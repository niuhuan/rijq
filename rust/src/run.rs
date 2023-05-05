use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use bytes::BytesMut;
use bytes::{Buf, BufMut};
use rand::seq::IteratorRandom;
use ricq::client::Token;
use ricq::ext::common::after_login;
use ricq::{
    LoginDeviceLocked, LoginNeedCaptcha, LoginResponse, LoginSuccess, LoginUnknownStatus,
    QRCodeConfirmed, QRCodeImageFetch, QRCodeState, RQError, RQResult,
};
use ricq_core::binary::BinaryReader;
use ricq_core::binary::BinaryWriter;
use std::cmp::min;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::sleep;

pub(crate) async fn run_ricq(c: Arc<ricq::Client>) -> Result<()> {
    tracing::info!("开始运行客户端");
    // 连接到服务器
    let mut handle = connection(c.clone()).await?;
    // 让步
    tokio::task::yield_now().await;
    sleep(Duration::from_secs(1)).await;
    // 连接成功
    tracing::info!("已连接到服务器");
    // 优先使用token登录
    if !token_login(c.as_ref()).await {
        tracing::info!("未能使用token登录，进行扫码");
        qr_login(c.clone()).await?;
        write_token_to_store(c.gen_token().await).await?;
    }
    loop {
        // 每次轮询d
        after_login(&c.clone()).await;
        // 直到连接断开
        tracing::info!("开始接收消息");
        let err = match handle.await {
            Ok(_) => {
                tracing::warn!("连接已断开");
                anyhow::Error::msg("what's up")
            }
            Err(err) => {
                tracing::warn!("连接已断开 {:?}", err);
                err.into()
            }
        };
        handle = re_connection(c.clone()).await?;
        // 让步
        tokio::task::yield_now().await;
        sleep(Duration::from_secs(1)).await;
        // 连接成功
        tracing::info!("恢复连接");
        if token_login(c.as_ref()).await {
            tracing::info!("恢复会话");
        } else {
            tracing::warn!("未能恢复会话");
            return Err(err);
        }
    }
}

async fn connection(client: Arc<ricq::Client>) -> Result<JoinHandle<()>> {
    let addresses = client.get_address_list().await;
    let address = addresses
        .into_iter()
        .choose_stable(&mut rand::thread_rng())
        .unwrap();
    let conn = TcpStream::connect(address)
        .await
        .with_context(|| "连接到服务器出错")?;
    Ok(tokio::spawn(async move { client.start(conn).await }))
}

async fn token_login(client: &ricq::Client) -> bool {
    let session_file = FileSessionStore::boxed("rijq.session");
    let session_data = match session_file.load_session().await {
        Ok(data) => data,
        Err(err) => {
            tracing::info!("{:?}", err);
            return false;
        }
    };
    if let Some(session_data) = session_data {
        let result = client.token_login(bytes_to_token(session_data)).await;
        match result {
            Ok(_) => true,
            Err(err) => match err {
                RQError::TokenLoginFailed => {
                    // token error (KickedOffline)
                    let _ = session_file.remove_session().await;
                    false
                }
                _ => false,
            },
        }
    } else {
        false
    }
}

async fn write_token_to_store(token: Token) -> Result<()> {
    let session_file = FileSessionStore::boxed("rijq.session");
    session_file
        .as_ref()
        .save_session(token_to_bytes(&token).to_vec())
        .await
}

async fn qr_login(rq_client: Arc<ricq::Client>) -> Result<()> {
    let mut image_sig = Bytes::new();
    let mut resp = rq_client
        .fetch_qrcode()
        .await
        .map_err(|e| anyhow!("二维码加载失败  : {:?}", e))?;
    tracing::info!("获取到二维码 : {:?}", image_sig);
    loop {
        match resp {
            QRCodeState::ImageFetch(QRCodeImageFetch {
                ref image_data,
                ref sig,
            }) => {
                image_sig = sig.clone();
                if let Err(err) = print_qr_to_console(image_data) {
                    return Err(anyhow!("二维码打印到控制台时出现误 : {}", err));
                }
                tracing::info!("请扫码");
            }
            QRCodeState::WaitingForScan => {
                // tracing::info!("二维码待扫描")
            }
            QRCodeState::WaitingForConfirm => {
                // tracing::info!("二维码待确认")
            }
            QRCodeState::Timeout => {
                tracing::info!("二维码已超时，重新获取");
                resp = rq_client
                    .fetch_qrcode()
                    .await
                    .with_context(|| "二维码加载失败")?;
                continue;
            }
            QRCodeState::Confirmed(QRCodeConfirmed {
                ref tmp_pwd,
                ref tmp_no_pic_sig,
                ref tgt_qr,
                ..
            }) => {
                tracing::info!("二维码已确认");
                let first = rq_client
                    .qrcode_login(tmp_pwd, tmp_no_pic_sig, tgt_qr)
                    .await;
                return loop_login(rq_client, first).await;
            }
            QRCodeState::Canceled => {
                return Err(anyhow::Error::msg("二维码已取消"));
            }
        }
        sleep(Duration::from_secs(5)).await;
        resp = rq_client
            .query_qrcode_result(&image_sig)
            .await
            .with_context(|| "二维码状态加载失败")?;
    }
}

#[async_trait::async_trait]
pub trait SessionStore {
    async fn save_session(&self, data: Vec<u8>) -> Result<()>;
    async fn load_session(&self) -> Result<Option<Vec<u8>>>;
    async fn remove_session(&self) -> Result<()>;
}

pub struct FileSessionStore {
    pub path: String,
}

impl FileSessionStore {
    pub fn boxed(path: impl Into<String>) -> Box<dyn SessionStore + Send + Sync> {
        return Box::new(Self { path: path.into() });
    }
}

#[async_trait::async_trait]
impl SessionStore for FileSessionStore {
    async fn save_session(&self, data: Vec<u8>) -> Result<()> {
        tokio::fs::write(self.path.as_str(), data).await?;
        Ok(())
    }
    async fn load_session(&self) -> Result<Option<Vec<u8>>> {
        if Path::new(self.path.as_str()).exists() {
            Ok(Some(tokio::fs::read(self.path.as_str()).await?))
        } else {
            Ok(None)
        }
    }
    async fn remove_session(&self) -> Result<()> {
        let _ = tokio::fs::remove_file(self.path.as_str()).await;
        Ok(())
    }
}

async fn loop_login(rq_client: Arc<ricq::Client>, first: RQResult<LoginResponse>) -> Result<()> {
    // netwotrk error
    let mut resp = first?;
    loop {
        match resp {
            LoginResponse::Success(LoginSuccess {
                ref account_info, ..
            }) => {
                tracing::info!("登录成功: {:?}", account_info);
                return Ok(());
            }
            LoginResponse::DeviceLocked(LoginDeviceLocked {
                ref sms_phone,
                ref verify_url,
                ref message,
                ..
            }) => {
                tracing::info!("设备锁 : {:?}", message);
                tracing::info!("密保手机 : {:?}", sms_phone);
                tracing::info!("验证地址 : {:?}", verify_url);
                qr2term::print_qr(
                    verify_url
                        .clone()
                        .with_context(|| "未能取得设备锁验证地址")?
                        .as_str(),
                )?;
                tracing::info!("验证地址 : {:?}", verify_url);
                tracing::info!("手机扫码或者打开url，处理完成后重启程序");
                std::process::exit(0);
            }
            LoginResponse::NeedCaptcha(LoginNeedCaptcha {
                ref verify_url,
                // 图片应该没了
                image_captcha: ref _image_captcha,
                ..
            }) => {
                tracing::info!("滑动条 (原URL) : {:?}", verify_url);
                let helper_url = verify_url
                    .clone()
                    .unwrap()
                    .replace("ssl.captcha.qq.com", "txhelper.glitch.me");
                tracing::info!("滑动条 (改URL) : {:?}", helper_url);
                let mut txt = http_get(&helper_url)
                    .await
                    .with_context(|| "http请求失败")?;
                tracing::info!("您需要使用该仓库 提供的APP进行滑动 , 滑动后请等待, https://github.com/mzdluo123/TxCaptchaHelper : {}", txt);
                loop {
                    sleep(Duration::from_secs(5)).await;
                    let rsp = http_get(&helper_url)
                        .await
                        .with_context(|| "http请求失败")?;
                    if !rsp.eq(&txt) {
                        txt = rsp;
                        break;
                    }
                }
                tracing::info!("获取到ticket : {}", txt);
                resp = rq_client.submit_ticket(&txt).await.expect("发送ticket失败");
            }
            LoginResponse::DeviceLockLogin { .. } => {
                resp = rq_client
                    .device_lock_login()
                    .await
                    .with_context(|| "设备锁登录失败")?;
            }
            LoginResponse::AccountFrozen => {
                return Err(anyhow::Error::msg("账户被冻结"));
            }
            LoginResponse::TooManySMSRequest => {
                return Err(anyhow::Error::msg("短信请求过于频繁"));
            }
            LoginResponse::UnknownStatus(LoginUnknownStatus {
                ref status,
                ref tlv_map,
                message,
                ..
            }) => {
                return Err(anyhow::Error::msg(format!(
                    "不能解析的登录响应: {:?}, {:?}, {:?}",
                    status, tlv_map, message,
                )));
            }
        }
    }
}

async fn http_get(url: &str) -> Result<String> {
    Ok(reqwest::ClientBuilder::new().build().unwrap().get(url).header(
        "user-agent", "Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Mobile Safari/537.36",
    ).send().await?
        .text()
        .await?)
}

fn print_qr_to_console(buff: &Bytes) -> Result<()> {
    let img = image::load_from_memory(buff)?.into_luma8();
    let mut img = rqrr::PreparedImage::prepare(img);
    let grids = img.detect_grids();
    let (_, content) = grids.get(0).with_context(|| "未能识别出二维码")?.decode()?;
    qr2term::print_qr(content.as_str())?;
    Ok(())
}

pub fn token_to_bytes(t: &Token) -> Bytes {
    let mut token = BytesMut::with_capacity(1024);
    token.put_i64(t.uin);
    token.write_bytes_short(&t.d2);
    token.write_bytes_short(&t.d2key);
    token.write_bytes_short(&t.tgt);
    token.write_bytes_short(&t.srm_token);
    token.write_bytes_short(&t.t133);
    token.write_bytes_short(&t.encrypted_a1);
    token.write_bytes_short(&t.wt_session_ticket_key);
    token.write_bytes_short(&t.out_packet_session_id);
    token.write_bytes_short(&t.tgtgt_key);
    token.freeze()
}

pub fn bytes_to_token(token: Vec<u8>) -> Token {
    let mut t = Bytes::from(token);
    Token {
        uin: t.get_i64(),
        d2: t.read_bytes_short().to_vec(),
        d2key: t.read_bytes_short().to_vec(),
        tgt: t.read_bytes_short().to_vec(),
        srm_token: t.read_bytes_short().to_vec(),
        t133: t.read_bytes_short().to_vec(),
        encrypted_a1: t.read_bytes_short().to_vec(),
        wt_session_ticket_key: t.read_bytes_short().to_vec(),
        out_packet_session_id: t.read_bytes_short().to_vec(),
        tgtgt_key: t.read_bytes_short().to_vec(),
    }
}

async fn re_connection(client: Arc<ricq::Client>) -> Result<JoinHandle<()>> {
    let mut times = 0;
    loop {
        times += 1;
        let d = Duration::from_secs(1) + Duration::from_secs(min(5, times - 1));
        tracing::info!("{}秒后进行{}次重连", d.as_secs(), times);
        sleep(d).await;
        let res = connection(client.clone()).await;
        match res {
            Ok(jh) => return Ok(jh),
            Err(_) => (),
        }
    }
}

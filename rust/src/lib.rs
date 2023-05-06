use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::run::run_ricq;
use jni::objects::{JByteArray, JClass, JObject, JString};
use jni::signature::{Primitive, ReturnType};
use jni::sys::jlong;
use jni::JNIEnv;
use prost::Message;
use ricq::handler::QEvent;
use ricq::version::ANDROID_WATCH;
use ricq_core::msg::elem::{FlashImage, RQElem};
use ricq_core::msg::MessageChain;
use ricq_core::protocol::device::Device;
use tokio::runtime::Runtime;

mod obj {
    include!(concat!(env!("OUT_DIR"), "/obj.rs"));
    pub(crate) use super::enums;
}
mod enums {
    include!(concat!(env!("OUT_DIR"), "/enums.rs"));
}
mod log;
mod run;

struct JHandler {
    sender: Arc<tokio::sync::mpsc::UnboundedSender<QEvent>>,
}

#[async_trait::async_trait]
impl ricq::handler::Handler for JHandler {
    async fn handle(&self, event: QEvent) {
        self.sender.send(event).unwrap();
    }
}

#[no_mangle]
#[allow(unused_mut)]
pub extern "system" fn Java_rijq_framework_handlers_InitRunner_daemon(
    mut env: JNIEnv,
    runner: JObject,
) {
    log::init_log_once();
    // 提示daemon启动
    tracing::info!("daemon start");
    // 启动runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_keep_alive(Duration::from_secs(100))
        .worker_threads(10)
        .max_blocking_threads(10)
        .build()
        .unwrap();
    tracing::info!("runtime init");
    // 初始化channel，启动ricq
    let (sender, mut r) = tokio::sync::mpsc::unbounded_channel::<QEvent>();
    let sender = Arc::new(sender);
    let device = runtime.block_on(device());
    let client = ricq::Client::new(
        device,
        ANDROID_WATCH,
        JHandler {
            sender: sender.clone(),
        },
    );
    let client = Arc::new(client);
    let c1 = client.clone();
    let _ = runtime.spawn(async move { run_ricq(c1, sender).await.unwrap() });
    // 获取env,runtime,client的指针, 并传递给InitRunner
    let env_point = &env as *const JNIEnv as i64;
    println!("got env_point : {env_point}");
    let runtime_point = &runtime as *const Runtime as i64;
    println!("got runtime_point : {runtime_point}");
    let client_point = &client as *const Arc<ricq::Client> as i64;
    println!("got client_point : {client_point}");
    env.call_method(
        &runner,
        "setEnvPoints",
        "(JJJ)V",
        &[
            jni::objects::JValue::Long(env_point),
            jni::objects::JValue::Long(runtime_point),
            jni::objects::JValue::Long(client_point),
        ],
    )
    .unwrap();
    println!("EnvPoints set");
    // 获取runner的dispatchEventMethodPoint方法
    let runner_class = env.get_object_class(&runner).unwrap();
    tracing::info!("got runner class");
    let dispatch_method = env
        .get_method_id(
            &runner_class,
            "dispatchEventMethodPoint",
            "(Ljava/lang/Object;)V",
        )
        .unwrap();
    // 获取LoginEvent类
    let login_event_class = env.find_class("rijq/framework/obj/LoginEvent").unwrap();
    let group_message_class = env
        .find_class("rijq/framework/obj/GroupMessageEvent")
        .unwrap();
    let friend_message_class = env
        .find_class("rijq/framework/obj/FriendMessageEvent")
        .unwrap();
    // 开始接收事件
    while let Some(event) = runtime.block_on(r.recv()) {
        println!("event : {:?}", event);
        match event {
            QEvent::Login(uid) => {
                let event = obj::LoginEvent { uid };
                let data = event.encode_to_vec();
                let data = env.byte_array_from_slice(data.as_slice()).unwrap();
                let de = env
                    .call_static_method(
                        &login_event_class,
                        "parseFrom",
                        "([B)Lrijq/framework/obj/LoginEvent;",
                        &[(&data).into()],
                    )
                    .unwrap();
                unsafe {
                    env.call_method_unchecked(
                        &runner,
                        &dispatch_method,
                        ReturnType::Primitive(Primitive::Void),
                        &[de.as_jni()],
                    )
                    .unwrap();
                }
            }
            QEvent::GroupMessage(gm) => {
                let inner = gm.inner;
                let event = obj::GroupMessageEvent {
                    seqs: inner.seqs,
                    rands: inner.rands,
                    group_code: inner.group_code,
                    group_name: inner.group_name,
                    group_card: inner.group_card,
                    from_uin: inner.from_uin,
                    time: inner.time,
                    elements: map_elements(inner.elements),
                };
                let data = event.encode_to_vec();
                let data = env.byte_array_from_slice(data.as_slice()).unwrap();
                let de = env
                    .call_static_method(
                        &group_message_class,
                        "parseFrom",
                        "([B)Lrijq/framework/obj/GroupMessageEvent;",
                        &[(&data).into()],
                    )
                    .unwrap();
                unsafe {
                    env.call_method_unchecked(
                        &runner,
                        &dispatch_method,
                        ReturnType::Primitive(Primitive::Void),
                        &[de.as_jni()],
                    )
                    .unwrap();
                }
            }
            QEvent::FriendMessage(fm) => {
                let inner = fm.inner;
                let event = obj::FriendMessageEvent {
                    seqs: inner.seqs,
                    rands: inner.rands,
                    from_uin: inner.from_uin,
                    time: inner.time,
                    elements: map_elements(inner.elements),
                    target: inner.target,
                    from_nick: inner.from_nick,
                };
                let data = event.encode_to_vec();
                let data = env.byte_array_from_slice(data.as_slice()).unwrap();
                let de = env
                    .call_static_method(
                        &friend_message_class,
                        "parseFrom",
                        "([B)Lrijq/framework/obj/FriendMessageEvent;",
                        &[(&data).into()],
                    )
                    .unwrap();
                unsafe {
                    env.call_method_unchecked(
                        &runner,
                        &dispatch_method,
                        ReturnType::Primitive(Primitive::Void),
                        &[de.as_jni()],
                    )
                    .unwrap();
                }
            }
            QEvent::GroupAudioMessage(_) => {}
            QEvent::FriendAudioMessage(_) => {}
            QEvent::GroupTempMessage(_) => {}
            QEvent::GroupRequest(_) => {}
            QEvent::SelfInvited(_) => {}
            QEvent::NewFriendRequest(_) => {}
            QEvent::NewMember(_) => {}
            QEvent::GroupMute(_) => {}
            QEvent::FriendMessageRecall(_) => {}
            QEvent::GroupMessageRecall(_) => {}
            QEvent::NewFriend(_) => {}
            QEvent::GroupLeave(_) => {}
            QEvent::GroupDisband(_) => {}
            QEvent::FriendPoke(_) => {}
            QEvent::GroupPoke(_) => {}
            QEvent::GroupNameUpdate(_) => {}
            QEvent::DeleteFriend(_) => {}
            QEvent::MemberPermissionChange(_) => {}
            QEvent::KickedOffline(_) => {}
            QEvent::MSFOffline(_) => {}
            QEvent::ClientDisconnect(_) => {}
        }
    }
    let _ = client;
    // let user2 = unsafe { &mut *(env_point as *mut jni::JNIEnv) };
    // user2.find_class("/").unwrap();
}

async fn device() -> Device {
    let file_name = "device.json";
    if Path::new(file_name).exists() {
        serde_json::from_str(&tokio::fs::read_to_string(file_name).await.unwrap()).unwrap()
    } else {
        let device = Device::random();
        tokio::fs::write(file_name, serde_json::to_string(&device).unwrap())
            .await
            .unwrap();
        device
    }
}

#[no_mangle]
#[allow(unused_mut)]
pub extern "system" fn Java_rijq_framework_handlers_InitRunner_callNative(
    mut _env: JNIEnv,
    _class: JClass,
    _env_point: jlong,
    _runtime_point: jlong,
    _client_point: jlong,
    _message_type: JString,
    _message: JByteArray,
) {
    println!("callNative : {_env_point}, {_runtime_point}, {_client_point}");
    let runtime = unsafe { &*(_runtime_point as *const Runtime) };
    let client = unsafe { &*(_client_point as *const Arc<ricq::Client>) };
    runtime.spawn(async {});
    println!("123");
    // _message_type 转换成 str
    let message_type: String = _env
        .get_string(&_message_type)
        .expect("Couldn't get java string!")
        .into();
    // _message 转换成 Vec<u8>
    let message: Vec<u8> = _env
        .convert_byte_array(_message)
        .expect("Couldn't get java byte array!")
        .into();
    match message_type.as_str() {
        "SendFriendMessage" => {
            let message: obj::SendFriendMessage =
                obj::SendFriendMessage::decode(&mut Cursor::new(message))
                    .expect("SendFriendMessage decode error");
            let target = message.target;
            let message = map_send(message.elements);
            runtime
                .block_on(async move { client.send_friend_message(target, message).await })
                .expect("send friend message error");
        }
        _ => {}
    }
}

fn map_elements(chain: MessageChain) -> Vec<obj::MessageElement> {
    let mut vc = vec![];
    for element in chain {
        match element {
            RQElem::At(at) => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::At),
                    element_data: obj::At {
                        target: at.target,
                        display: at.display,
                    }
                    .encode_to_vec(),
                });
            }
            RQElem::Text(text) => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::Text),
                    element_data: obj::Text {
                        content: text.content,
                    }
                    .encode_to_vec(),
                });
            }
            RQElem::Face(face) => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::Face),
                    element_data: obj::Face {
                        index: face.index,
                        name: face.name,
                    }
                    .encode_to_vec(),
                });
            }
            RQElem::MarketFace(market_face) => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::MarketFace),
                    element_data: obj::MarketFace {
                        name: market_face.name,
                        face_id: market_face.face_id,
                        tab_id: market_face.tab_id,
                        item_type: market_face.item_type,
                        sub_type: market_face.sub_type,
                        media_type: market_face.media_type,
                        encrypt_key: market_face.encrypt_key,
                        magic_value: market_face.magic_value,
                    }
                    .encode_to_vec(),
                });
            }
            RQElem::Dice(dice) => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::Dice),
                    element_data: obj::Dice { value: dice.value }.encode_to_vec(),
                });
            }
            RQElem::FriendImage(friend_image) => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::FriendImage),
                    element_data: obj::FriendImage {
                        res_id: friend_image.res_id,
                        file_path: friend_image.file_path,
                        md5: friend_image.md5,
                        size: friend_image.size,
                        width: friend_image.width,
                        height: friend_image.height,
                        image_type: friend_image.image_type,
                        orig_url: friend_image.orig_url,
                        download_path: friend_image.download_path,
                        flash: false,
                    }
                    .encode_to_vec(),
                });
            }
            RQElem::GroupImage(group_image) => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::GroupImage),
                    element_data: obj::GroupImage {
                        file_path: group_image.file_path,
                        file_id: group_image.file_id,
                        size: group_image.size,
                        width: group_image.width,
                        height: group_image.height,
                        md5: group_image.md5,
                        orig_url: group_image.orig_url.unwrap_or_default(),
                        image_type: group_image.image_type,
                        signature: group_image.signature,
                        server_ip: group_image.server_ip,
                        server_port: group_image.server_port,
                        flash: false,
                    }
                    .encode_to_vec(),
                });
            }
            RQElem::FlashImage(flash_image) => match flash_image {
                FlashImage::FriendImage(friend_image) => {
                    vc.push(obj::MessageElement {
                        element_type: i32::from(obj::enums::ElementType::FriendImage),
                        element_data: obj::FriendImage {
                            res_id: friend_image.res_id,
                            file_path: friend_image.file_path,
                            md5: friend_image.md5,
                            size: friend_image.size,
                            width: friend_image.width,
                            height: friend_image.height,
                            image_type: friend_image.image_type,
                            orig_url: friend_image.orig_url,
                            download_path: friend_image.download_path,
                            flash: true,
                        }
                        .encode_to_vec(),
                    });
                }
                FlashImage::GroupImage(group_image) => {
                    vc.push(obj::MessageElement {
                        element_type: i32::from(obj::enums::ElementType::GroupImage),
                        element_data: obj::GroupImage {
                            file_path: group_image.file_path,
                            file_id: group_image.file_id,
                            size: group_image.size,
                            width: group_image.width,
                            height: group_image.height,
                            md5: group_image.md5,
                            orig_url: group_image.orig_url.unwrap_or_default(),
                            image_type: group_image.image_type,
                            signature: group_image.signature,
                            server_ip: group_image.server_ip,
                            server_port: group_image.server_port,
                            flash: true,
                        }
                        .encode_to_vec(),
                    });
                }
            },
            RQElem::VideoFile(video_file) => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::VideoFile),
                    element_data: obj::VideoFile {
                        name: video_file.name,
                        uuid: video_file.uuid,
                        size: video_file.size,
                        thumb_size: video_file.thumb_size,
                        md5: video_file.md5,
                        thumb_md5: video_file.thumb_md5,
                    }
                    .encode_to_vec(),
                });
            }
            _ => {
                vc.push(obj::MessageElement {
                    element_type: i32::from(obj::enums::ElementType::Unknown),
                    element_data: vec![],
                });
            }
        }
    }
    vc
}

fn map_send(elements: Vec<obj::MessageElement>) -> MessageChain {
    let mut chain = MessageChain::default();
    for x in elements {
        if x.element_type == i32::from(obj::enums::ElementType::Text) {
            let text = obj::Text::decode(&mut Cursor::new(x.element_data)).expect("Text decode");
            chain.push(ricq_core::msg::elem::Text::new(text.content));
        }
    }
    chain
}

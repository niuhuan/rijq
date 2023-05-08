RIJQ
====

QQ机器人框架, 完美融合Spring生态。基于RICQ。

项目还在婴儿阶段，仅有一些demo功能，并不能使用，很欢迎PR。


## 如何使用

### 新建一个Spring项目，并增加context

```java
@SpringBootApplication
public class RunnerApplication {
	public static void main(String[] args) {
		new SpringApplication(RunnerApplication.class).run(args);
	}
}
```

### 增加一个组件

- 登录时打印ID
- 收到群消息时打印群名
- 收到好友消息时回复hello

```java
@Module
@Slf4j
@RequiredArgsConstructor
public class LogModule {
    private final JQClient jqClient;
    @Handler
    public boolean onLogin(LoginEvent a) {
        log.info("onLogin: {}", a.getUid());
        return true;
    }
    @Handler
    public boolean onGroupMessage(GroupMessageEvent groupMessageEvent) {
        log.info("groupMessageEvent: {}", groupMessageEvent.getGroupName());
        return true;
    }
    @Handler
    public boolean onFriendMessage(FriendMessageEvent friendMessageEvent) {
        log.info("friendMessageEvent: {}", friendMessageEvent.getFromNick());
        jqClient.sendFriendMessage(friendMessageEvent.getFromUin(), "hello");
        return true;
    }
}
```

## 如何运行demo

环境准备

1. rust nightly latest
2. java 17 (Spring3)

在项目运行以下命令

```shell
make dev
```

## 功能

- [x] 登录
- [x] 发个人文字信息
- [x] 接收消息
- [ ] 还有好多没做

## 提示

Module、Event 都是可以增加@Order注解了
Event的返回值，如果是true表示拦截，false表示继续传递

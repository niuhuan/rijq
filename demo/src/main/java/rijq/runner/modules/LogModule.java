package rijq.runner.modules;

import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import rijq.framework.annotaions.Handler;
import rijq.framework.annotaions.Module;
import rijq.framework.handlers.JQClient;
import rijq.framework.obj.FriendMessageEvent;
import rijq.framework.obj.GroupMessageEvent;
import rijq.framework.obj.LoginEvent;

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

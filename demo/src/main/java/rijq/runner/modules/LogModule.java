package rijq.runner.modules;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import rijq.framework.annotaions.Handler;
import rijq.framework.annotaions.Module;
import rijq.framework.handlers.JQClient;
import rijq.framework.obj.FriendMessageEvent;
import rijq.framework.obj.GroupMessageEvent;
import rijq.framework.obj.LoginEvent;

@Module
public class LogModule {
    private final Logger logger = LoggerFactory.getLogger(getClass());
    private final JQClient jqClient;
    public LogModule(JQClient jqClient) {
        this.jqClient = jqClient;
    }
    @Handler
    public boolean onLogin(LoginEvent a) {
        logger.info("onLogin: {}", a.getUid());
        return true;
    }
    @Handler
    public boolean onGroupMessage(GroupMessageEvent groupMessageEvent) {
        logger.info("groupMessageEvent: {}", groupMessageEvent.getGroupName());
        return true;
    }
    @Handler
    public boolean onFriendMessage(FriendMessageEvent friendMessageEvent) {
        logger.info("friendMessageEvent: {}", friendMessageEvent.getFromNick());
        jqClient.sendFriendMessage(friendMessageEvent.getFromUin(), "hello");
        return true;
    }
}

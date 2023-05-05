package rijq.framework.handlers;

import org.springframework.stereotype.Component;
import rijq.framework.obj.MessageElement;
import rijq.framework.obj.SendFriendMessage;
import rijq.framework.obj.Text;

@Component
public class JQClient {

    private final InitRunner initRunner;

    public JQClient(InitRunner initRunner) {
        this.initRunner = initRunner;
    }

    public void sendFriendMessage(
            long uin,
            String text
    ) {
        initRunner.callNative(
                "SendFriendMessage",
                SendFriendMessage.newBuilder()
                        .setTarget(uin)
                        .addElements(MessageElement.newBuilder()
                                .setElementType("Text")
                                .setElementData(Text.newBuilder().setContent(text).build().toByteString()))
                        .build().toByteArray()
        );
    }

}

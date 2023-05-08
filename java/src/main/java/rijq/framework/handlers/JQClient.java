package rijq.framework.handlers;

import com.google.protobuf.ByteString;
import lombok.SneakyThrows;
import org.springframework.stereotype.Component;
import rijq.framework.obj.*;
import rijq.framework.obj.enums.ElementType;
import rijq.framework.obj.enums.SendTargetType;

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
                                .setElementType(ElementType.Text)
                                .setElementData(Text.newBuilder().setContent(text).build().toByteString()))
                        .build().toByteArray()
        );
    }


    @SneakyThrows
    public FriendImage uploadFriendImage(
            long uin,
            byte[] buff
    ) {
        var result = initRunner.callNative(
                "UploadImage",
                UploadImageDto.newBuilder()
                        .setTargetType(SendTargetType.Friend)
                        .setTarget(uin)
                        .setData(ByteString.copyFrom(buff))
                        .build().toByteArray()
        );
        return FriendImage.parseFrom(result);
    }


    @SneakyThrows
    public FriendImage uploadGropImage(
            long groupNumber,
            byte[] buff
    ) {
        var result = initRunner.callNative(
                "UploadImage",
                UploadImageDto.newBuilder()
                        .setTargetType(SendTargetType.Group)
                        .setTarget(groupNumber)
                        .setData(ByteString.copyFrom(buff))
                        .build().toByteArray()
        );
        return FriendImage.parseFrom(result);
    }

}

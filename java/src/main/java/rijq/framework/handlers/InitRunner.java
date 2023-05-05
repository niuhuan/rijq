package rijq.framework.handlers;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.ApplicationArguments;
import org.springframework.boot.ApplicationRunner;
import org.springframework.context.ApplicationContext;
import org.springframework.core.annotation.Order;
import org.springframework.stereotype.Component;
import rijq.framework.annotaions.Handler;
import rijq.framework.annotaions.Module;
import rijq.framework.obj.FriendMessageEvent;
import rijq.framework.obj.GroupMessageEvent;
import rijq.framework.obj.LoginEvent;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.*;

@Component
public class InitRunner implements ApplicationRunner {

    static {
        System.loadLibrary("rijq");
    }

    private final Logger logger = LoggerFactory.getLogger(getClass());


    private final ApplicationContext applicationContext;
    private final Map<Class, List<EventMethodPoint>> points;


    public static class EventMethodPoint {
        public Module module;
        public Class moduleClass;
        public Object moduleInstance;
        public String name;
        public Method method;
    }

    public InitRunner(ApplicationContext applicationContext) {
        this.applicationContext = applicationContext;
        this.points = new HashMap<>();
    }

    @Override
    public void run(ApplicationArguments args) throws Exception {
        var moduleBeans = getModuleBeans();
        putPoints(LoginEvent.class, moduleBeans);
        putPoints(GroupMessageEvent.class, moduleBeans);
        putPoints(FriendMessageEvent.class, moduleBeans);
        this.daemon();
        // sendMessage(env_point, runtime_point, "1",  LoginEvent.getDefaultInstance().toByteArray());
    }

    private void putPoints(Class<?> clazz, List<Object> moduleBeans) throws Exception {
        if (!points.containsKey(clazz)) {
            points.put(clazz, new ArrayList<>());
        } else {
            throw new RuntimeException("EventMethodPoint already exists");
        }
        for (Object moduleBean : moduleBeans) {
            var methods = moduleBean.getClass().getMethods();
            var handlerMethods = Arrays.stream(methods).filter(method -> method.getAnnotation(Handler.class) != null).sorted((m1, m2) -> {
                int order1 = m1.getAnnotation(Order.class) != null ? m1.getAnnotation(Order.class).value() : 0;
                int order2 = m2.getAnnotation(Order.class) != null ? m2.getAnnotation(Order.class).value() : 0;
                return Integer.compare(order1, order2);
            }).toList();
            for (Method method : handlerMethods) {
                if (method.getParameterCount() != 1) {
                    throw new RuntimeException("参数最多为1个");
                }
                if (method.getReturnType() != boolean.class) {
                    throw new RuntimeException("返回值必须为 boolean 类型");
                }
                var parameterType = method.getParameterTypes()[0];
                if (parameterType == clazz) {
                    var point = new EventMethodPoint();
                    point.module = moduleBean.getClass().getAnnotation(Module.class);
                    point.moduleClass = moduleBean.getClass();
                    point.moduleInstance = moduleBean;
                    point.method = method;
                    points.get(clazz).add(point);
                    logger.info("注册事件处理器: {} -> {} ({})", point.moduleClass, method.getName(), clazz.getName());
                }
            }
        }
    }


    private List<Object> getModuleBeans() {
        var beansWithModuleAnnotation = applicationContext.getBeansWithAnnotation(Module.class);
        List<Object> moduleBeans = new ArrayList<>(beansWithModuleAnnotation.values());
        moduleBeans.sort(new Comparator<Object>() {
            @Override
            public int compare(Object o1, Object o2) {
                int order1 = o1.getClass().getAnnotation(Order.class) != null ? o1.getClass().getAnnotation(Order.class).value() : 0;
                int order2 = o2.getClass().getAnnotation(Order.class) != null ? o2.getClass().getAnnotation(Order.class).value() : 0;
                return Integer.compare(order1, order2);
            }
        });
        return moduleBeans;
    }

    public void dispatchEventMethodPoint(Object object) throws InvocationTargetException, IllegalAccessException {
        var list = points.get(object.getClass());
        if (list != null) {
            for (EventMethodPoint eventMethodPoint : list) {
                if ((Boolean) (eventMethodPoint.method.invoke(eventMethodPoint.moduleInstance, object))) {
                    // todo log
                    return;
                }
            }
        }
    }

    private long env_point;
    private long runtime_point;
    private long client_point;

    private void setEnvPoints(long env_point, long runtime_point, long client_point) {
        this.env_point = env_point;
        this.runtime_point = runtime_point;
        this.client_point = client_point;
    }

    private native void callNative(long env_point, long runtime_point, long client_point, String messageType, byte[] message);

    protected void callNative(String messageType, byte[] message) {
        callNative(
                this.env_point,
                this.runtime_point,
                this.client_point,
                messageType,
                message
        );
    }

    private native void daemon();

}

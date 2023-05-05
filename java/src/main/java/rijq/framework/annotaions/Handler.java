package rijq.framework.annotaions;

import java.lang.annotation.*;

@Target(ElementType.METHOD)
@Retention(RetentionPolicy.RUNTIME)
@Documented
public @interface Handler {
    String id() default "";

    String name() default "";
}

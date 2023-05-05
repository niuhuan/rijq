package rijq.runner;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;

@SpringBootApplication
public class RunnerApplication {

	public static void main(String[] args) {
		new SpringApplication(RunnerApplication.class).run(args);
	}

}

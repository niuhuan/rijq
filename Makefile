#!make

.PHONY: all

dev:
	cd rust && cargo build
	cd demo && ./gradlew bootJar && java -Djava.library.path=../rust/target/debug/ -jar build/libs/runner.jar

clean:
	cd rust && cargo clean
	cd demo && ./gradlew clean

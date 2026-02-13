package com.svnsearch;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.scheduling.annotation.EnableScheduling;

@SpringBootApplication
@EnableScheduling
public class SvnSearchApplication {
    public static void main(String[] args) {
        SpringApplication.run(SvnSearchApplication.class, args);
    }
}

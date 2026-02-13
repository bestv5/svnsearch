package com.svnsearch.service;

import com.svnsearch.config.AppConfig;
import org.springframework.stereotype.Service;
import java.io.*;
import java.util.*;

@Service
public class SvnClient {
    private final AppConfig config;
    
    public SvnClient(AppConfig config) {
        this.config = config;
    }
    
    public SvnResult testConnection(String url, String username, String password) {
        List<String> cmd = buildCommand(Arrays.asList("info", url, "--xml"), username, password);
        return executeCommand(cmd);
    }
    
    public SvnResult listDirectory(String url, String username, String password) {
        List<String> cmd = buildCommand(Arrays.asList("list", url, "--xml", "-R"), username, password);
        return executeCommand(cmd);
    }
    
    public SvnResult getFileContent(String url, String username, String password) {
        List<String> cmd = buildCommand(Arrays.asList("cat", url), username, password);
        return executeCommand(cmd);
    }
    
    private List<String> buildCommand(List<String> args, String username, String password) {
        List<String> cmd = new ArrayList<>();
        cmd.add(config.getSvnPath());
        cmd.addAll(args);
        cmd.add("--non-interactive");
        cmd.add("--trust-server-cert-failures=unknown-ca");
        if (username != null && !username.isEmpty()) {
            cmd.add("--username");
            cmd.add(username);
        }
        if (password != null && !password.isEmpty()) {
            cmd.add("--password");
            cmd.add(password);
        }
        return cmd;
    }
    
    private SvnResult executeCommand(List<String> cmd) {
        try {
            ProcessBuilder pb = new ProcessBuilder(cmd);
            pb.redirectErrorStream(true);
            Process process = pb.start();
            
            StringBuilder output = new StringBuilder();
            try (BufferedReader reader = new BufferedReader(new InputStreamReader(process.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    output.append(line).append("\n");
                }
            }
            
            int exitCode = process.waitFor();
            return new SvnResult(exitCode == 0, output.toString(), exitCode);
        } catch (Exception e) {
            return new SvnResult(false, e.getMessage(), -1);
        }
    }
    
    public static class SvnResult {
        private final boolean success;
        private final String output;
        private final int exitCode;
        
        public SvnResult(boolean success, String output, int exitCode) {
            this.success = success;
            this.output = output;
            this.exitCode = exitCode;
        }
        
        public boolean isSuccess() { return success; }
        public String getOutput() { return output; }
        public int getExitCode() { return exitCode; }
    }
}

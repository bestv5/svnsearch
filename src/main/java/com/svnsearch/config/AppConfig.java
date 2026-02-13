package com.svnsearch.config;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.context.annotation.Configuration;

@Configuration
@ConfigurationProperties(prefix = "svnsearch")
public class AppConfig {
    private String dataDir = "./data";
    private int updateInterval = 60;
    private String svnPath = "svn";

    public String getDataDir() { return dataDir; }
    public void setDataDir(String dataDir) { this.dataDir = dataDir; }
    public int getUpdateInterval() { return updateInterval; }
    public void setUpdateInterval(int updateInterval) { this.updateInterval = updateInterval; }
    public String getSvnPath() { return svnPath; }
    public void setSvnPath(String svnPath) { this.svnPath = svnPath; }
}

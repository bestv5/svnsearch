package com.svnsearch.controller;

import com.svnsearch.config.AppConfig;
import com.svnsearch.model.Repository;
import com.svnsearch.model.FileIndex;
import com.svnsearch.repository.RepositoryRepository;
import com.svnsearch.repository.FileIndexRepository;
import com.svnsearch.service.SvnClient;
import com.svnsearch.service.IndexService;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;
import java.util.*;

@RestController
@RequestMapping("/api")
@CrossOrigin(origins = "*")
public class ApiController {
    
    private final RepositoryRepository repoRepo;
    private final FileIndexRepository fileRepo;
    private final SvnClient svnClient;
    private final IndexService indexService;
    private final AppConfig config;
    
    public ApiController(RepositoryRepository repoRepo, FileIndexRepository fileRepo, 
                         SvnClient svnClient, IndexService indexService, AppConfig config) {
        this.repoRepo = repoRepo;
        this.fileRepo = fileRepo;
        this.svnClient = svnClient;
        this.indexService = indexService;
        this.config = config;
    }
    
    @GetMapping("/config")
    public Map<String, Object> getConfig() {
        Map<String, Object> result = new HashMap<>();
        result.put("data_dir", config.getDataDir());
        result.put("update_interval", config.getUpdateInterval());
        result.put("svn_path", config.getSvnPath());
        result.put("repositories", repoRepo.findAll());
        return result;
    }
    
    @PostMapping("/config")
    public Map<String, Object> updateConfig(@RequestBody Map<String, Object> configMap) {
        if (configMap.containsKey("data_dir")) {
            config.setDataDir((String) configMap.get("data_dir"));
        }
        if (configMap.containsKey("update_interval")) {
            config.setUpdateInterval((Integer) configMap.get("update_interval"));
        }
        if (configMap.containsKey("svn_path")) {
            config.setSvnPath((String) configMap.get("svn_path"));
        }
        Map<String, Object> result = new HashMap<>();
        result.put("success", true);
        return result;
    }
    
    @GetMapping("/repositories")
    public List<Repository> getRepositories() {
        return repoRepo.findAll();
    }
    
    @PostMapping("/repositories")
    public Map<String, Object> addRepository(@RequestBody Map<String, String> data) {
        Map<String, Object> result = new HashMap<>();
        
        String name = data.get("name");
        String url = data.get("url");
        String username = data.get("username");
        String password = data.get("password");
        
        if (name == null || url == null || name.isEmpty() || url.isEmpty()) {
            result.put("success", false);
            result.put("error", "Name and URL are required");
            return result;
        }
        
        SvnClient.SvnResult testResult = svnClient.testConnection(url, username, password);
        if (!testResult.isSuccess()) {
            result.put("success", false);
            result.put("error", "Connection failed: " + testResult.getOutput());
            return result;
        }
        
        Repository repo = new Repository();
        repo.setName(name);
        repo.setUrl(url);
        repo.setUsername(username);
        repo.setPassword(password);
        repo = repoRepo.save(repo);
        
        result.put("success", true);
        result.put("repo_id", repo.getId());
        return result;
    }
    
    @DeleteMapping("/repositories/{id}")
    public Map<String, Object> deleteRepository(@PathVariable Long id) {
        Map<String, Object> result = new HashMap<>();
        fileRepo.deleteByRepoId(id);
        repoRepo.deleteById(id);
        result.put("success", true);
        return result;
    }
    
    @PostMapping("/repositories/{id}/index")
    public Map<String, Object> indexRepository(@PathVariable Long id) {
        Map<String, Object> result = new HashMap<>();
        new Thread(() -> indexService.indexRepository(id)).start();
        result.put("success", true);
        return result;
    }
    
    @GetMapping("/repositories/{id}/status")
    public IndexService.IndexStatus getIndexStatus(@PathVariable Long id) {
        return indexService.getStatus(id);
    }
    
    @GetMapping("/search")
    public List<Map<String, Object>> search(@RequestParam String q, 
                                             @RequestParam(required = false) Long repoId,
                                             @RequestParam(defaultValue = "1000") int limit) {
        List<FileIndex> files;
        if (repoId != null) {
            files = fileRepo.findByFilenameContainingIgnoreCaseAndRepoId(q, repoId);
        } else {
            files = fileRepo.findByFilenameContainingIgnoreCase(q);
        }
        
        List<Map<String, Object>> result = new ArrayList<>();
        for (FileIndex file : files) {
            if (result.size() >= limit) break;
            Optional<Repository> optRepo = repoRepo.findById(file.getRepoId());
            if (optRepo.isPresent()) {
                Repository repo = optRepo.get();
                Map<String, Object> item = new HashMap<>();
                item.put("id", file.getId());
                item.put("path", file.getPath());
                item.put("filename", file.getFilename());
                item.put("is_dir", file.isDir());
                item.put("size", file.getSize());
                item.put("repo_id", file.getRepoId());
                item.put("repo_name", repo.getName());
                item.put("repo_url", repo.getUrl());
                item.put("full_url", repo.getUrl() + "/" + file.getPath());
                result.add(item);
            }
        }
        return result;
    }
    
    @GetMapping("/file/content")
    public Map<String, Object> getFileContent(@RequestParam String url, @RequestParam Long repoId) {
        Map<String, Object> result = new HashMap<>();
        Optional<Repository> optRepo = repoRepo.findById(repoId);
        if (optRepo.isEmpty()) {
            result.put("error", "Repository not found");
            return result;
        }
        
        Repository repo = optRepo.get();
        SvnClient.SvnResult svnResult = svnClient.getFileContent(url, repo.getUsername(), repo.getPassword());
        
        if (!svnResult.isSuccess()) {
            result.put("error", svnResult.getOutput());
            return result;
        }
        
        String content = svnResult.getOutput();
        if (content.length() > 100000) {
            content = content.substring(0, 100000);
        }
        
        result.put("type", "text");
        result.put("content", content);
        return result;
    }
    
    @GetMapping("/stats")
    public Map<String, Object> getStats() {
        Map<String, Object> result = new HashMap<>();
        result.put("repository_count", repoRepo.count());
        result.put("total_files", fileRepo.countTotal());
        
        List<Map<String, Object>> repos = new ArrayList<>();
        for (Repository repo : repoRepo.findAll()) {
            Map<String, Object> r = new HashMap<>();
            r.put("id", repo.getId());
            r.put("name", repo.getName());
            r.put("last_update", repo.getLastUpdate());
            r.put("file_count", fileRepo.countByRepoId(repo.getId()));
            repos.add(r);
        }
        result.put("repositories", repos);
        
        return result;
    }
}

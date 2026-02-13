package com.svnsearch.service;

import com.svnsearch.model.Repository;
import com.svnsearch.model.FileIndex;
import com.svnsearch.repository.RepositoryRepository;
import com.svnsearch.repository.FileIndexRepository;
import org.springframework.scheduling.annotation.Scheduled;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import javax.xml.parsers.DocumentBuilder;
import javax.xml.parsers.DocumentBuilderFactory;
import org.w3c.dom.*;
import java.io.StringReader;
import org.xml.sax.InputSource;
import java.time.LocalDateTime;
import java.util.*;
import java.util.concurrent.*;

@Service
public class IndexService {
    private final RepositoryRepository repoRepo;
    private final FileIndexRepository fileRepo;
    private final SvnClient svnClient;
    private final Map<Long, IndexStatus> indexStatus = new ConcurrentHashMap<>();
    
    public IndexService(RepositoryRepository repoRepo, FileIndexRepository fileRepo, SvnClient svnClient) {
        this.repoRepo = repoRepo;
        this.fileRepo = fileRepo;
        this.svnClient = svnClient;
    }
    
    public IndexStatus getStatus(Long repoId) {
        return indexStatus.getOrDefault(repoId, new IndexStatus("idle", 0, ""));
    }
    
    @Transactional
    public void indexRepository(Long repoId) {
        Optional<Repository> optRepo = repoRepo.findById(repoId);
        if (optRepo.isEmpty()) return;
        
        Repository repo = optRepo.get();
        indexStatus.put(repoId, new IndexStatus("indexing", 0, "Starting..."));
        
        try {
            fileRepo.deleteByRepoId(repoId);
            
            SvnClient.SvnResult result = svnClient.listDirectory(repo.getUrl(), repo.getUsername(), repo.getPassword());
            
            if (!result.isSuccess()) {
                indexStatus.put(repoId, new IndexStatus("error", 0, result.getOutput()));
                return;
            }
            
            List<FileIndex> files = parseListOutput(result.getOutput(), repoId, repo.getUrl());
            int total = files.size();
            int processed = 0;
            
            List<FileIndex> batch = new ArrayList<>();
            for (FileIndex file : files) {
                batch.add(file);
                if (batch.size() >= 1000) {
                    fileRepo.saveAll(batch);
                    batch.clear();
                }
                processed++;
                if (processed % 100 == 0) {
                    indexStatus.put(repoId, new IndexStatus("indexing", processed * 100 / total, 
                        "Indexed " + processed + "/" + total + " files"));
                }
            }
            
            if (!batch.isEmpty()) {
                fileRepo.saveAll(batch);
            }
            
            repo.setLastUpdate(LocalDateTime.now());
            repoRepo.save(repo);
            
            indexStatus.put(repoId, new IndexStatus("completed", 100, "Completed: " + total + " files indexed"));
            
        } catch (Exception e) {
            indexStatus.put(repoId, new IndexStatus("error", 0, e.getMessage()));
        }
    }
    
    private List<FileIndex> parseListOutput(String xml, Long repoId, String baseUrl) {
        List<FileIndex> files = new ArrayList<>();
        try {
            DocumentBuilder builder = DocumentBuilderFactory.newInstance().newDocumentBuilder();
            Document doc = builder.parse(new InputSource(new StringReader(xml)));
            NodeList entries = doc.getElementsByTagName("entry");
            
            for (int i = 0; i < entries.getLength(); i++) {
                Element entry = (Element) entries.item(i);
                String path = entry.getAttribute("path");
                String kind = entry.getAttribute("kind");
                
                Element sizeElem = (Element) entry.getElementsByTagName("size").item(0);
                long size = sizeElem != null ? Long.parseLong(sizeElem.getTextContent()) : 0;
                
                Element commit = (Element) entry.getElementsByTagName("commit").item(0);
                long revision = commit != null ? Long.parseLong(commit.getAttribute("revision")) : 0;
                
                String filename = path.isEmpty() ? path : path.substring(path.lastIndexOf('/') + 1);
                if (filename.isEmpty() && !path.isEmpty()) {
                    filename = path;
                }
                
                FileIndex file = new FileIndex();
                file.setRepoId(repoId);
                file.setPath(path);
                file.setFilename(filename);
                file.setDir("dir".equals(kind));
                file.setSize(size);
                file.setRevision(revision);
                files.add(file);
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
        return files;
    }
    
    @Scheduled(fixedDelayString = "${svnsearch.update-interval:3600000}")
    public void autoIndexAll() {
        List<Repository> repos = repoRepo.findAll();
        for (Repository repo : repos) {
            indexRepository(repo.getId());
        }
    }
    
    public static class IndexStatus {
        private final String status;
        private final int progress;
        private final String message;
        
        public IndexStatus(String status, int progress, String message) {
            this.status = status;
            this.progress = progress;
            this.message = message;
        }
        
        public String getStatus() { return status; }
        public int getProgress() { return progress; }
        public String getMessage() { return message; }
    }
}

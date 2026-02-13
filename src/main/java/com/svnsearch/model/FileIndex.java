package com.svnsearch.model;

import jakarta.persistence.*;
import java.time.LocalDateTime;

@Entity
@Table(name = "file_index", indexes = {
    @Index(name = "idx_filename", columnList = "filename"),
    @Index(name = "idx_path", columnList = "path"),
    @Index(name = "idx_repo_id", columnList = "repoId")
})
public class FileIndex {
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;
    
    @Column(name = "repo_id")
    private Long repoId;
    
    private String path;
    private String filename;
    
    @Column(name = "is_dir")
    private boolean isDir;
    
    private Long size;
    private Long revision;
    
    @Column(name = "last_modified")
    private LocalDateTime lastModified;
    
    @Column(name = "indexed_at")
    private LocalDateTime indexedAt;
    
    @PrePersist
    protected void onCreate() {
        indexedAt = LocalDateTime.now();
    }
    
    public Long getId() { return id; }
    public void setId(Long id) { this.id = id; }
    public Long getRepoId() { return repoId; }
    public void setRepoId(Long repoId) { this.repoId = repoId; }
    public String getPath() { return path; }
    public void setPath(String path) { this.path = path; }
    public String getFilename() { return filename; }
    public void setFilename(String filename) { this.filename = filename; }
    public boolean isDir() { return isDir; }
    public void setDir(boolean isDir) { this.isDir = isDir; }
    public Long getSize() { return size; }
    public void setSize(Long size) { this.size = size; }
    public Long getRevision() { return revision; }
    public void setRevision(Long revision) { this.revision = revision; }
    public LocalDateTime getLastModified() { return lastModified; }
    public void setLastModified(LocalDateTime lastModified) { this.lastModified = lastModified; }
    public LocalDateTime getIndexedAt() { return indexedAt; }
    public void setIndexedAt(LocalDateTime indexedAt) { this.indexedAt = indexedAt; }
}

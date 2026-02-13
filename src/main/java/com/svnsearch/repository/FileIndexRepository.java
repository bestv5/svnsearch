package com.svnsearch.repository;

import com.svnsearch.model.FileIndex;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Query;
import org.springframework.data.repository.query.Param;
import java.util.List;

public interface FileIndexRepository extends JpaRepository<FileIndex, Long> {
    List<FileIndex> findByFilenameContainingIgnoreCase(String filename);
    
    List<FileIndex> findByFilenameContainingIgnoreCaseAndRepoId(String filename, Long repoId);
    
    @Query("SELECT COUNT(f) FROM FileIndex f")
    long countTotal();
    
    @Query("SELECT COUNT(f) FROM FileIndex f WHERE f.repoId = :repoId")
    long countByRepoId(@Param("repoId") Long repoId);
    
    void deleteByRepoId(Long repoId);
}

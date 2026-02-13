package com.svnsearch.repository;

import com.svnsearch.model.Repository;
import org.springframework.data.jpa.repository.JpaRepository;
import java.util.Optional;

public interface RepositoryRepository extends JpaRepository<Repository, Long> {
    Optional<Repository> findByUrl(String url);
}

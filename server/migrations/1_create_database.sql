-- ============================================================================
-- Database Setup per Ruggine Chat Application
-- ============================================================================
-- Questo script crea:
-- 1. Il database principale 'rugginedb'
-- 2. Un utente dedicato 'ruggine' con privilegi limitati
-- 3. Tutte le tabelle e indici necessari
--
-- NOTA: Eseguire questo script come utente root MySQL
-- CREDENZIALI (devono corrispondere al file .env):
--   - Username: ruggine
--   - Password: ferro
--   - Database: rugginedb
--   - Connection: mysql://ruggine:ferro@127.0.0.1:3306/rugginedb
-- ============================================================================

-- Creazione database
CREATE DATABASE IF NOT EXISTS `rugginedb` 
    DEFAULT CHARACTER SET utf8mb4 
    COLLATE utf8mb4_unicode_ci;

-- Creazione utente dedicato per l'applicazione
-- Creiamo l'utente per localhost, 127.0.0.1 e % (accesso remoto)
CREATE USER IF NOT EXISTS 'ruggine'@'localhost' IDENTIFIED BY 'ferro';
CREATE USER IF NOT EXISTS 'ruggine'@'127.0.0.1' IDENTIFIED BY 'ferro';
CREATE USER IF NOT EXISTS 'ruggine'@'%' IDENTIFIED BY 'ferro';

-- Assegnazione privilegi: solo quelli necessari (principio del minimo privilegio)
-- SELECT, INSERT, UPDATE, DELETE per operazioni CRUD
-- CREATE, INDEX, ALTER per gestione schema (migrazioni future)
GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, INDEX, ALTER ON rugginedb.* TO 'ruggine'@'localhost';
GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, INDEX, ALTER ON rugginedb.* TO 'ruggine'@'127.0.0.1';
GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, INDEX, ALTER ON rugginedb.* TO 'ruggine'@'%';

-- Applica i privilegi
FLUSH PRIVILEGES;

USE `rugginedb`;
-- MySQL dump 10.13  Distrib 8.0.43, for Win64 (x86_64)
--
-- Host: localhost    Database: rugginedb
-- ------------------------------------------------------
-- Server version	8.0.43

/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;
/*!50503 SET NAMES utf8 */;
/*!40103 SET @OLD_TIME_ZONE=@@TIME_ZONE */;
/*!40103 SET TIME_ZONE='+00:00' */;
/*!40014 SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0 */;
/*!40014 SET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0 */;
/*!40101 SET @OLD_SQL_MODE=@@SQL_MODE, SQL_MODE='NO_AUTO_VALUE_ON_ZERO' */;
/*!40111 SET @OLD_SQL_NOTES=@@SQL_NOTES, SQL_NOTES=0 */;

--
-- Table structure for table `chats`
--

DROP TABLE IF EXISTS `chats`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `chats` (
  `chat_id` int NOT NULL AUTO_INCREMENT,
  `title` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `description` text COLLATE utf8mb4_unicode_ci,
  `chat_type` enum('GROUP','PRIVATE') COLLATE utf8mb4_unicode_ci NOT NULL,
  PRIMARY KEY (`chat_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `invitations`
--

DROP TABLE IF EXISTS `invitations`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `invitations` (
  `invite_id` int NOT NULL AUTO_INCREMENT,
  `target_chat_id` int NOT NULL,
  `invited_id` int NOT NULL,
  `invitee_id` int NOT NULL,
  `state` enum('PENDING','ACCEPTED','REJECTED') COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'PENDING',
  `created_at` timestamp NOT NULL,
  PRIMARY KEY (`invite_id`),
  UNIQUE KEY `uq_Invitations_group_user_status` (`target_chat_id`,`invited_id`,`state`),
  KEY `idx_Invitations_group` (`target_chat_id`),
  KEY `idx_Invitations_invited_user` (`invited_id`),
  KEY `invitations_ibfk_3` (`invitee_id`),
  CONSTRAINT `invitations_ibfk_1` FOREIGN KEY (`target_chat_id`) REFERENCES `chats` (`chat_id`) ON DELETE CASCADE,
  CONSTRAINT `invitations_ibfk_2` FOREIGN KEY (`invited_id`) REFERENCES `users` (`user_id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `messages`
--

DROP TABLE IF EXISTS `messages`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `messages` (
  `message_id` int NOT NULL AUTO_INCREMENT,
  `chat_id` int NOT NULL,
  `sender_id` int NOT NULL,
  `content` text COLLATE utf8mb4_unicode_ci NOT NULL,
  `message_type` enum('USERMESSAGE','SYSTEMMESSAGE') COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT 'USERMESSAGE',
  `created_at` timestamp NOT NULL,
  PRIMARY KEY (`message_id`),
  KEY `idx_Messages_chat_createdAt` (`chat_id`,`created_at` DESC),
  KEY `idx_Messages_sender` (`sender_id`),
  CONSTRAINT `messages_ibfk_1` FOREIGN KEY (`chat_id`) REFERENCES `chats` (`chat_id`) ON DELETE CASCADE,
  CONSTRAINT `messages_ibfk_2` FOREIGN KEY (`sender_id`) REFERENCES `users` (`user_id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `userchatmetadata`
--

DROP TABLE IF EXISTS `userchatmetadata`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `userchatmetadata` (
  `user_id` int NOT NULL,
  `chat_id` int NOT NULL,
  `messages_visible_from` timestamp NOT NULL,
  `messages_received_until` timestamp NOT NULL,
  `user_role` enum('OWNER','ADMIN','MEMBER') COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `member_since` timestamp NOT NULL,
  PRIMARY KEY (`chat_id`,`user_id`),
  KEY `idx_UCM_user` (`user_id`),
  KEY `idx_UCM_chat` (`chat_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Table structure for table `users`
--

DROP TABLE IF EXISTS `users`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `users` (
  `user_id` int NOT NULL AUTO_INCREMENT,
  `username` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `password` text COLLATE utf8mb4_unicode_ci NOT NULL,
  PRIMARY KEY (`user_id`),
  UNIQUE KEY `username` (`username`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
/*!40101 SET character_set_client = @saved_cs_client */;
/*!40103 SET TIME_ZONE=@OLD_TIME_ZONE */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;

-- Dump completed on 2025-10-05 23:39:27

-- Grant permissions to ruggine user for creating test databases
GRANT CREATE ON *.* TO 'ruggine'@'localhost';
GRANT DROP ON *.* TO 'ruggine'@'localhost';
GRANT ALL PRIVILEGES ON `_sqlx_test_%`.* TO 'ruggine'@'127.0.0.1';
FLUSH PRIVILEGES;

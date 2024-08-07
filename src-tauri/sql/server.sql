DROP TABLE IF EXISTS `server`;
CREATE TABLE `server` (
  `id` varchar(200) NOT NULL,
  `ip` varchar(45) DEFAULT NULL,
  `port` varchar(45) DEFAULT NULL,
  `account` varchar(200) DEFAULT NULL,
  `pwd` varchar(200) DEFAULT NULL,
  `name` varchar(200) DEFAULT NULL,
  `description` varchar(200) DEFAULT NULL,
  `create_date` varchar(200) DEFAULT NULL,
  `update_date` varchar(200) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;

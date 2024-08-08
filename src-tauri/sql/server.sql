-- ----------------------------
-- Table structure for server
-- ----------------------------
DROP TABLE IF EXISTS `server`;
CREATE TABLE `server` (
  `id` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci NOT NULL,
  `ip` varchar(20) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `port` int DEFAULT NULL,
  `account` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `pwd` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `name` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `description` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `create_time` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `update_time` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  PRIMARY KEY (`id`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3;

-- ----------------------------
-- Table structure for pipeline_tag
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_tag`;
CREATE TABLE `pipeline_tag` (
  `id` varchar(255) NOT NULL,
  `name` varchar(255) DEFAULT NULL,
  `value` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Records of pipeline_tag
-- ----------------------------
BEGIN;
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('1', '开发', 'develop', '2024-08-07 10:40:19', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('2', '测试', 'test', '2024-08-07 10:40:20', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('3', 'C++', 'C++', '2024-08-07 10:40:21', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('4', 'Rust', 'Rust', '2024-08-07 10:40:22', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('5', 'Java', 'Java', '2024-08-07 10:40:23', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('6', 'Android', 'Android', '2024-08-07 10:40:24', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('7', 'Ios', 'Ios', '2024-08-07 10:40:25', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('8', 'H5', 'H5', '2024-08-07 10:40:26', NULL);
COMMIT;


-- ----------------------------
-- Table structure for pipeline
-- ----------------------------
DROP TABLE IF EXISTS `pipeline`;
CREATE TABLE `pipeline` (
  `id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL,
  `server_id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '服务器 ID',
  `last_run_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '最后运行时间',
  `status` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '当前运行状态',
  `basic_id` varchar(255) DEFAULT NULL,
  `process_id` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_basic
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_basic`;
CREATE TABLE `pipeline_basic` (
  `id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL,
  `pipeline_id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '流水线ID',
  `name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '名称',
  `tag` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '标签',
  `path` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '项目路径',
  `description` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '描述',
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_group
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_group`;
CREATE TABLE `pipeline_group` (
  `id` varchar(255) NOT NULL,
  `stage_id` varchar(255) DEFAULT NULL,
  `title` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_process
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_process`;
CREATE TABLE `pipeline_process` (
  `id` varchar(255) NOT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_stage
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_stage`;
CREATE TABLE `pipeline_stage` (
  `id` varchar(255) NOT NULL,
  `process_id` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_step
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_step`;
CREATE TABLE `pipeline_step` (
  `id` varchar(255) NOT NULL,
  `group_id` varchar(255) DEFAULT NULL,
  `module` varchar(255) DEFAULT NULL,
  `command` varchar(255) DEFAULT NULL,
  `label` varchar(255) DEFAULT NULL,
  `status` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_step_component
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_step_component`;
CREATE TABLE `pipeline_step_component` (
  `id` varchar(255) NOT NULL,
  `step_id` varchar(255) DEFAULT NULL,
  `prop` varchar(255) DEFAULT NULL,
  `value` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_tag
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_tag`;
CREATE TABLE `pipeline_tag` (
  `id` varchar(255) NOT NULL,
  `name` varchar(255) DEFAULT NULL,
  `value` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_variable
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_variable`;
CREATE TABLE `pipeline_variable` (
  `id` varchar(255) DEFAULT NULL,
  `pipeline_id` varchar(255) DEFAULT NULL,
  `name` varchar(255) DEFAULT NULL COMMENT '变量名',
  `genre` varchar(255) DEFAULT NULL COMMENT '变量类型',
  `value` varchar(255) DEFAULT NULL COMMENT '值',
  `disabled` varchar(255) DEFAULT NULL COMMENT '是否禁用',
  `require` varchar(255) DEFAULT NULL COMMENT '是否必填',
  `description` varchar(500) DEFAULT NULL COMMENT '描述',
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for server
-- ----------------------------
DROP TABLE IF EXISTS `server`;
CREATE TABLE `server` (
  `id` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci NOT NULL,
  `ip` varchar(20) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `port` int DEFAULT NULL,
  `account` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `pwd` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `name` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `description` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `create_time` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  `update_time` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci DEFAULT NULL,
  PRIMARY KEY (`id`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3;

SET FOREIGN_KEY_CHECKS = 1;

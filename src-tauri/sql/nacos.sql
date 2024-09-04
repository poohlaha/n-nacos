/*
 Navicat Premium Data Transfer

 Source Server         : localhost
 Source Server Type    : MySQL
 Source Server Version : 80039 (8.0.39)
 Source Host           : localhost:3306
 Source Schema         : nacos

 Target Server Type    : MySQL
 Target Server Version : 80039 (8.0.39)
 File Encoding         : 65001

 Date: 04/09/2024 17:17:46
*/

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

-- ----------------------------
-- Table structure for article
-- ----------------------------
DROP TABLE IF EXISTS `article`;
CREATE TABLE `article` (
  `id` varchar(255) NOT NULL,
  `title` varchar(255) DEFAULT NULL COMMENT '标题',
  `content` longtext COMMENT '内容',
  `tag_ids` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '标签id, 多个用`,` 分割',
  `create_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  `update_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for article_tag
-- ----------------------------
DROP TABLE IF EXISTS `article_tag`;
CREATE TABLE `article_tag` (
  `id` varchar(255) NOT NULL,
  `name` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  `update_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline
-- ----------------------------
DROP TABLE IF EXISTS `pipeline`;
CREATE TABLE `pipeline` (
  `id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL,
  `server_id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '服务器 ID',
  `tag_id` varchar(255) DEFAULT NULL,
  `last_run_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '最后运行时间',
  `last_run_id` varchar(255) DEFAULT NULL COMMENT '最后运行ID',
  `status` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '当前运行状态',
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
  `tag_id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '标签',
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
  `label` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  `order` int DEFAULT NULL,
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
  `pipeline_id` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_runtime
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_runtime`;
CREATE TABLE `pipeline_runtime` (
  `id` varchar(255) NOT NULL,
  `pipeline_id` varchar(255) DEFAULT NULL COMMENT '流水线ID',
  `project_name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '项目名称，根据 url 获取',
  `order` int DEFAULT NULL COMMENT '顺序',
  `tag` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '标签',
  `basic` longtext,
  `stages` longtext COMMENT 'stages',
  `status` varchar(255) DEFAULT NULL COMMENT '运行状态',
  `start_time` varchar(255) DEFAULT NULL COMMENT '开始时间',
  `duration` varchar(255) DEFAULT NULL COMMENT '运行时长, 单位秒',
  `stage_index` int DEFAULT NULL COMMENT 'stage 运行到哪一步, 从 1 开始计算',
  `group_index` int DEFAULT NULL COMMENT 'group 运行到哪一步, 从 0 开始计算',
  `step_index` int DEFAULT NULL COMMENT 'step 运行到哪一步, 从 0 开始计算',
  `finished` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '是否完成',
  `remark` varchar(255) DEFAULT NULL COMMENT '运行备注',
  `log` varchar(500) DEFAULT NULL COMMENT '日志文件地址',
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_runtime_snapshot
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_runtime_snapshot`;
CREATE TABLE `pipeline_runtime_snapshot` (
  `id` varchar(255) NOT NULL,
  `runtime_id` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '运行记录ID',
  `node` varchar(255) DEFAULT NULL COMMENT 'nodeJs 版本号',
  `branch` varchar(255) DEFAULT NULL COMMENT '分支',
  `make` varchar(255) DEFAULT NULL COMMENT 'Make 命令',
  `command` varchar(255) DEFAULT NULL COMMENT '本机安装的命令',
  `script` varchar(255) DEFAULT NULL COMMENT 'package.json 中的 scripts 命令',
  `create_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  `update_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  PRIMARY KEY (`id`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_runtime_variable
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_runtime_variable`;
CREATE TABLE `pipeline_runtime_variable` (
  `id` varchar(255) NOT NULL,
  `snapshot_id` varchar(255) DEFAULT NULL,
  `order` int DEFAULT NULL,
  `name` varchar(255) DEFAULT NULL,
  `value` varchar(255) DEFAULT NULL,
  `genre` varchar(255) DEFAULT NULL,
  `require` varchar(255) DEFAULT NULL,
  `disabled` varchar(255) DEFAULT NULL,
  `description` varchar(500) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT '描述',
  `create_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  `update_time` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  PRIMARY KEY (`id`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_stage
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_stage`;
CREATE TABLE `pipeline_stage` (
  `id` varchar(255) NOT NULL,
  `process_id` varchar(255) DEFAULT NULL,
  `order` int DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Table structure for pipeline_status
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_status`;
CREATE TABLE `pipeline_status` (
  `id` varchar(255) NOT NULL,
  `name` varchar(255) DEFAULT NULL,
  `value` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Records of pipeline_status
-- ----------------------------
BEGIN;
INSERT INTO `pipeline_status` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('1', 'No', 'No', '2024-08-13 15:13:01', NULL);
INSERT INTO `pipeline_status` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('2', 'Queue', 'Queue', '2024-08-13 15:13:01', NULL);
INSERT INTO `pipeline_status` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('3', 'Process', 'Process', '2024-08-13 15:13:01', NULL);
INSERT INTO `pipeline_status` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('4', 'Success', 'Success', '2024-08-13 15:13:01', NULL);
INSERT INTO `pipeline_status` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('5', 'Failed', 'Failed', '2024-08-13 15:13:01', NULL);
INSERT INTO `pipeline_status` (`id`, `name`, `value`, `create_time`, `update_time`) VALUES ('6', 'Stop', 'Stop', '2024-08-13 15:13:01', NULL);
COMMIT;

-- ----------------------------
-- Table structure for pipeline_step
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_step`;
CREATE TABLE `pipeline_step` (
  `id` varchar(255) NOT NULL,
  `group_id` varchar(255) DEFAULT NULL,
  `order` int DEFAULT NULL,
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
  `order` int DEFAULT NULL,
  `prop` varchar(255) DEFAULT NULL,
  `label` varchar(255) DEFAULT NULL,
  `value` longtext CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci,
  `description` varchar(255) DEFAULT NULL,
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
  `color` varchar(255) DEFAULT NULL,
  `create_time` varchar(255) DEFAULT NULL,
  `update_time` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

-- ----------------------------
-- Records of pipeline_tag
-- ----------------------------
BEGIN;
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('1', '开发', 'develop', 'processing', '2024-08-07 10:40:19', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('2', '测试', 'test', 'orange', '2024-08-07 10:40:20', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('3', 'C++', 'C++', 'lime', '2024-08-07 10:40:21', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('4', 'Rust', 'Rust', 'gold', '2024-08-07 10:40:22', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('5', 'Java', 'Java', 'purple', '2024-08-07 10:40:23', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('6', 'Android', 'Android', 'volcano', '2024-08-07 10:40:24', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('7', 'Ios', 'Ios', 'cyan', '2024-08-07 10:40:25', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('8', 'H5', 'H5', 'success', '2024-08-07 10:40:26', NULL);
INSERT INTO `pipeline_tag` (`id`, `name`, `value`, `color`, `create_time`, `update_time`) VALUES ('9', 'Docker-H5', 'DockerH5', 'magenta', '2024-08-07 10:40:27', NULL);
COMMIT;

-- ----------------------------
-- Table structure for pipeline_variable
-- ----------------------------
DROP TABLE IF EXISTS `pipeline_variable`;
CREATE TABLE `pipeline_variable` (
  `id` varchar(255) DEFAULT NULL,
  `pipeline_id` varchar(255) DEFAULT NULL,
  `order` int DEFAULT NULL COMMENT '顺序',
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

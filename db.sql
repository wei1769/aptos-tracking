-- Active: 1717371691760@@127.0.0.1@3306@defaultdb
CREATE TABLE `user` (
    `user_id` bigint unsigned NOT NULL,
    `user_name` varchar(20) DEFAULT NULL,
    `create_time` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`user_id`)
);

CREATE TABLE `wallet_tracked` (
    `wallet_id` bigint unsigned NOT NULL AUTO_INCREMENT,
    `chat_id` bigint NOT NULL,
    `user_id` bigint unsigned NOT NULL,
    `wallet_address` char(70) NOT NULL,
    `nickname` varchar(20) DEFAULT NULL,
    `track_type` enum(
        'full',
        'sent',
        'balance',
        'receive'
    ) NOT NULL DEFAULT 'full',
    `create_time` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `minimum_value` double NOT NULL DEFAULT '0',
    PRIMARY KEY (`wallet_id`),
    UNIQUE KEY `chat_id` (`chat_id`, `wallet_address`),
    KEY `address` (`wallet_address`)
);

CREATE TABLE `token_info` (
    `mint` varchar(70) NOT NULL,
    `name` varchar(70) DEFAULT NULL,
    `value` double NOT NULL DEFAULT '0',
    `decimal` tinyint unsigned NOT NULL DEFAULT '0',
    `is_skipped` tinyint unsigned NOT NULL DEFAULT '0',
    `update_time` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    `creat_time` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `last_updated_block` bigint unsigned NOT NULL DEFAULT '0',
    KEY `is_skipped` (`is_skipped`),
    PRIMARY KEY (`mint`)
);

CREATE TABLE `processed_block` (
    `block` bigint unsigned NOT NULL,
    `processed_time` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `last_tx_sig` varchar(100) DEFAULT NULL,
    `modulo` tinyint unsigned NOT NULL DEFAULT '1',
    `remainder` tinyint unsigned NOT NULL DEFAULT '0',
    `status` enum(
        'complete',
        'processing',
        'error',
        'skipped'
    ) NOT NULL DEFAULT 'processing',
    `place` enum('head', 'bottom', 'old') NOT NULL DEFAULT 'head',
    `tx_count` int unsigned NOT NULL DEFAULT '0',
    PRIMARY KEY (`block`),
    KEY `modulo_remainder_index` (`modulo`, `remainder`),
    KEY `place_index` (`place`),
    KEY `place_modulo_remainder_index` (
        `place`,
        `modulo`,
        `remainder`
    )
);

CREATE TABLE "processed_block" (
    "block" bigint unsigned NOT NULL,
    "processed_time" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "last_tx_sig" varchar(100) DEFAULT NULL,
    "modulo" tinyint unsigned NOT NULL DEFAULT '1',
    "remainder" tinyint unsigned NOT NULL DEFAULT '0',
    "status" enum(
        'complete',
        'processing',
        'error',
        'skipped'
    ) NOT NULL DEFAULT 'processing',
    "place" enum('head', 'bottom', 'old') NOT NULL DEFAULT 'head',
    "tx_count" int unsigned NOT NULL DEFAULT '0',
    PRIMARY KEY ("block"),
    KEY "modulo_remainder_index" ("modulo", "remainder"),
    KEY "place_index" ("place"),
    KEY "place_modulo_remainder_index" (
        "place",
        "modulo",
        "remainder"
    )
);

SELECT * FROM `wallet_last_tx`;

SELECT *, `user`.`user_name`
FROM `wallet_tracked`
    INNER JOIN `user` ON `user`.`user_id` = `wallet_tracked`.`user_id`;

SELECT wallet_tracked.wallet_address
FROM wallet_tracked
GROUP BY
    wallet_tracked.wallet_address;

SELECT `wallet_tracked`.`chat_id`, `wallet_tracked`.`wallet_address`, `wallet_tracked`.`track_type`, `wallet_last_tx`.`tx_sig`
FROM
    `wallet_tracked`
    INNER JOIN `wallet_last_tx` ON `wallet_last_tx`.`wallet_address` = `wallet_tracked`.`wallet_address`;

SELECT `wallet_tracked`.`wallet_address`, `wallet_last_tx`.`tx_sig`
FROM
    wallet_tracked
    INNER JOIN `wallet_last_tx` ON `wallet_last_tx`.`wallet_address` = `wallet_tracked`.`wallet_address`;

DESCRIBE wallet_last_tx;

ALTER TABLE `wallet_last_tx`
ALTER COLUMN last_block BIGINT UNSIGNED NOT NULL,;

select * from processed_block WHERE processed_block.`place` = 'head';

select block
FROM processed_block
WHERE
    processed_block.`block` > 224502911
ORDER BY block DESC
LIMIT 1000;

select *
from processed_block
WHERE
    processed_block.`status` = 'processing';

select MAX(processed_block.block) from processed_block;

SELECT MAX(`processed_block`.`block`) AS `block`
FROM `processed_block`
WHERE (
        `processed_block`.`block` = 224112492
    );
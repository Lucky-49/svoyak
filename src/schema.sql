BEGIN TRANSACTION;

DROP TABLE IF EXISTS "users_sessions";
CREATE TABLE IF NOT EXISTS "users_sessions" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "token" VARCHAR(255) NOT NULL
);

DROP TABLE IF EXISTS "users";
CREATE TABLE IF NOT EXISTS "users" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT,
    "role" VARCHAR(255) NOT NULL,
    "username" VARCHAR(255) NOT NULL,
    "password" VARCHAR(255) NOT NULL,
    "city" TEXT,
    "first_name_user" TEXT,
    "patronymic_user" TEXT,
    "last_name_user" TEXT,
    "phone_user" TEXT,
    "mail_user" TEXT,
    "created_user" TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    );



DROP TABLE IF EXISTS "register_questions_pac";
CREATE TABLE IF NOT EXISTS "register_questions_pac" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "package_name" TEXT NOT NULL,
    "stage" INTEGER,
    "created_package" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "type" TEXT
);

DROP TABLE IF EXISTS "players";
CREATE TABLE IF NOT EXISTS "players" (
    "player_id"	INTEGER NOT NULL PRIMARY KEY,
    "player_name"	TEXT,
    "player_first_name" TEXT,
    "player_last_name" TEXT,
    "chat_id" INTEGER,
    "created_player" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "player_real_first_name" TEXT,
    "player_real_patronymic" TEXT,
    "player_real_last_name" TEXT,
    "player_real_location" TEXT,
    "player_real_phone_number" INTEGER,
    "player_play_games" INTEGER DEFAULT 0,
    "player_win_games" INTEGER DEFAULT 0,
    "player_correct_answer" INTEGER DEFAULT 0,
    "player_incorrect_answer" INTEGER DEFAULT 0,
    "player_total_score" INTEGER DEFAULT 0,
    UNIQUE("player_id")
);


DROP TABLE IF EXISTS "questions_players";
CREATE TABLE IF NOT EXISTS "questions_players" (
    "id"	INTEGER PRIMARY KEY AUTOINCREMENT,
    "player_id" INTEGER NOT NULL,
    "player_topic_five_questions" TEXT,
    "player_question"	TEXT NOT NULL,
    "player_answer_question" TEXT NOT NULL,
    "package_id" INTEGER
);


DROP TABLE IF EXISTS "register_games";
CREATE TABLE IF NOT EXISTS "register_games" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "game_day" TEXT NOT NULL,
    "game_time" TEXT NOT NULL,
    "game_location" TEXT NOT NULL,
    "price_player"INTEGER,
    "price_spectator" INTEGER,
    "package_id" INTEGER,
    "stage" INTEGER,
    "seats_spectator" INTEGER
);

DROP TABLE IF EXISTS "pre_registrations_player";
CREATE TABLE IF NOT EXISTS "pre_registrations_player" (
    "id"	INTEGER PRIMARY KEY AUTOINCREMENT,
    "player_id"	INTEGER NOT NULL,
    "player_real_first_name" TEXT,
    "player_real_patronymic" TEXT,
    "player_real_last_name" TEXT,
    "player_real_location" TEXT,
    "player_real_phone_number" INTEGER
    );

DROP TABLE IF EXISTS "data_transfers";
CREATE TABLE IF NOT EXISTS "data_transfers" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT,
    "sender_user_id" INTEGER NOT NULL,
    "receiver_user_id" INTEGER NOT NULL,
    "transfer_date" DATETIME DEFAULT CURRENT_TIMESTAMP,
    "package_id" INTEGER NOT NULL,
    "right_transfer_other" INTEGER
    );

DROP TABLE IF EXISTS "message_id_del";
CREATE TABLE IF NOT EXISTS "message_id_del" (
    "player_id" INTEGER,
    "message_id" INTEGER
);

COMMIT;

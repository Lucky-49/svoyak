#![feature(let_chains)]
#[macro_use]
//говорит Rust о том, что макросы из крейта Rocket будут использоваться в текущем модуле.
extern crate rocket; //Этот макрос используется для подключения крейта Rocket.

#[macro_use]
extern crate serde_derive; /* Используется для подключения макросов десериализации и сериализации
                           из библиотеки serde_derive. Эти макросы позволяют автоматически генерировать код для
                           преобразования структур в JSON и обратно. */
extern crate rand; /* внешняя зависимость от библиотеки rand, которая предоставляет генераторы
                   случайных чисел. */

pub(crate) mod db;
mod tg;
mod web;

//подключение файла mod
use crate::web::server::rocket;
use db::*; //подключение файла db
use dotenv::dotenv;
use flexi_logger::{FileSpec, Logger};
use tg::bot::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok(); // Загрузка переменных окружения из файла .env

    // Создаем логгер
    let logger = Logger::try_with_env_or_str("info")
        .unwrap_or_else(|e| panic!("Failed to initialize logger: {:?}", e));

    // Настраиваем и активируем логирование в файл
    logger
        .log_to_file(
            FileSpec::default()
                .directory("src/logs")
                .basename("logs.txt"),
        )
        .format_for_files(flexi_logger::colored_detailed_format)
        .format_for_stderr(flexi_logger::colored_detailed_format)
        .start()
        .unwrap_or_else(|e| panic!("Failed to start logger: {:?}", e));

    // Теперь вы можете использовать logger для записи логов
    log::info!("Starting application...");
    log::error!("This is an error message");

    let _bot_task = tokio::spawn(async {
        //запускает телеграм-бота
        run_bot().await;
    });

    let _db_task = tokio::spawn(async {
        //запускаем базу данных
        run_db().await.expect("Ошибка запуска базы данных");
    });

    rocket().launch().await.expect("Ошибка запуска сервера");

    Ok(())
}

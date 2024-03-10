/* В этом файле содержатся содержаться структуры и функции, связанные с пользователями,
их созданием, регистрацией и авторизацией. */

use rocket::http::CookieJar; /* Импортируются структуры для работы с куки и
                             статусом HTTP из Rocket. */
use rusqlite::{params, OptionalExtension}; /* Импортируются функции и структуры из библиотеки
                                           rusqlite для взаимодействия с базой данных SQLite. */
use crate::db::establish_connection;
use rusqlite::{Connection, Error};

extern crate rand; /* внешняя зависимость от библиотеки rand, которая предоставляет генераторы
                   случайных чисел. */

#[derive(Serialize)] /* Атрибут derive используется для автоматической реализации определенных
                     трейтов для структуры или перечисления. В данном случае, Serialize говорит, что структура
                     Context может быть сериализована в формат JSON. Это полезно, например, при возврате данных в
                     формате JSON из веб-сервиса. */
pub struct Context {
    //объявление структуры Context
    pub header: String, //поле структуры
}

#[derive(Serialize)] /* Атрибут derive используется для автоматической реализации определенных
                     трайтов для структуры или перечисления. В данном случае, Serialize говорит, что структура
                     Context может быть сериализована в формат JSON. Это полезно, например, при возврате данных в
                     формате JSON из веб-сервиса. */
pub struct CSRFContext {
    //объявление структуры CSRFContext
    pub header: String,
    pub csrf: String,
}

#[derive(FromForm)] /* Атрибут derive с FromForm говорит Rocket, что структура UserFormSignup
                    может быть автоматически создана из данных формы (HTML формы) при запросе HTTP. */
pub struct UserFormSignup {
    //используется в fn register_user для получения логина и пароля со страницы signup
    pub username_signup: String,
    pub password_signup: String,
    pub city_signup: String,
    pub role_signup: String,
    pub first_name_user: String,
    pub patronymic_user: String,
    pub last_name_user: String,
    pub phone_user: String,
    pub mail_user: String,
    pub csrf: String,
}

#[derive(FromForm)] /* Атрибут derive с FromForm говорит Rocket, что структура UserFormSignup
                    может быть автоматически создана из данных формы (HTML формы) при запросе HTTP. */
pub struct UserFormLogin {
    //используется в fn login для получения логина и пароля со страницы index
    pub username_login: String,
    pub password_login: String,
    pub csrf: String,
}

#[derive(FromForm)]
pub struct CSRFtoken {
    //используется во всех веб формах
    pub csrf: String,
}

#[derive(Debug)]
pub struct UsersSession {
    //структура из таблицы users_session
    pub id_user_session: i64,
    pub user_id_users_session: i64,
    pub token_users_session: String,
}

#[derive(Debug)]
pub struct NewUserSession {
    //структура для таблицы users_session
    pub user_id_new_user_session: i64,
    pub token_new_user_session: String,
}

#[derive(Debug)]
pub struct UsersTable {
    //структура из таблицы users
    pub id_table: i64,
    pub username_table: String,
    pub password_table: String,
    pub city_table: String,
    pub role_table: String,
}

#[derive(Debug)]
pub struct NewUser {
    //структура для таблицы users
    pub username_new_user: String,
    pub password_new_user: String,
    pub city_new_user: String,
    pub role_new_user: String,
}

/* функция предназначена для извлечения user_id из базы данных по-заданному токену сессии
(session_token). */
pub fn get_user_id_from_session_token(session_token: String) -> Result<i64, std::io::Error> {
    let conn = establish_connection(); //устанавливаем соединение с бд

    let mut stmt = conn
        .prepare("SELECT user_id FROM users_sessions WHERE token = ? LIMIT 1") //создаем запрос в бд для поиска user_id по по токену
        .expect("не удалось выбрать значения fn get_user_id_from_session_token"); //обработка ошибки
    let user_id: Option<i64> = stmt //возвращаем user_id соответсвующий токену
        .query_row(params![session_token], |row| row.get(0))
        .optional()
        .expect("токен не найден fn get_user_id_from_session_token");

    match user_id {
        //Возвращает Result, где
        Some(id) => Ok(id), // Ok(id) содержит user_id, если токен найден
        None => Err(std::io::Error::new(
            // Err с информацией об ошибке в случае отсутствия токена
            std::io::ErrorKind::Other,
            "no token found",
        )),
    }
}

/* функция предназначена для получения user_id из cookie "session_token", используя функцию
get_user_id_from_session_token */
pub fn get_user_id_from_cookies(cookies: &CookieJar) -> Result<i64, std::io::Error> {
    match cookies.get_private("session_token") {
        //Извлекает cookie с именем "session_token" из переданного объекта cookies
        //Проверяет есть ли такой cookie (если Some(cookie)).
        Some(cookie) => match get_user_id_from_session_token(cookie.value().to_string()) {
            //Если cookie присутствует, вызывает функцию get_user_id_from_session_token с значением cookie в качестве аргумента
            Ok(user_id) => Ok(user_id), //Если get_user_id_from_session_token возвращает Ok(user_id), то функция возвращает Ok(user_id) в качестве результата.
            Err(error) => Err(error), //Если get_user_id_from_session_token возвращает ошибку (Err(error)), то функция возвращает ту же ошибку (Err(error)).
        },
        _ => {
            return Err(std::io::Error::new(
                //Если cookie отсутствует, функция возвращает ошибку std::io::Error с сообщением "no token found".
                std::io::ErrorKind::Other,
                "no token found",
            ));
        }
    }
}

//получение роли юзера
pub fn get_user_role(user_id: i64) -> Result<String, Error> {
    let conn = establish_connection();
    let mut stmt = conn
        .prepare("SELECT role FROM users WHERE id = ?")
        .expect("Не удалось подготовить запрос для получения роли пользователя");

    let role = stmt
        .query_row(params![user_id], |row| row.get(0))
        .unwrap_or_else(|err| {
            eprintln!(
                "Не удалось получить id пользователя из базы данных: {}",
                err
            );
            "visitor".to_string()
        });
    Ok(role)
}

/* Функция генерирует случайную строку в шестнадцатеричном формате определенной длины, которая
может быть использована в качестве сессионного токена. Возвращает результат в виде
Result<String, std::io::Error>. Если все операции выполнены успешно, возвращается
сгенерированная строка, иначе возвращается ошибка std::io::Error */
pub fn generate_session_token(length: u8) -> Result<String, std::io::Error> {
    let bytes: Vec<u8> = (0..length).map(|_| rand::random::<u8>()).collect(); //Генерирует коллекцию случайных байтов заданной длины length. Использует метод map для вызова rand::random::<u8>() length раз и преобразует результат в вектор типа u8.
    let strings: Vec<String> = bytes.iter().map(|byte| format!("{:02X}", byte)).collect(); //Преобразует каждый байт в строку, представляющую его шестнадцатеричное значение с двумя символами (в формате {:02X}). Использует метод map для применения форматирования к каждому байту и преобразует результат в вектор строк.
    return Ok(strings.join("")); //Соединяет все шестнадцатеричные строки в одну строку, используя метод join. Возвращает результат в виде Ok, упаковывая строку.
}

/* Функция выполняет запрос к базе данных, чтобы получить хеш пароля пользователя по его имени
(username). В случае успеха, она возвращает результат в виде Ok с полученным хешем пароля.
Если пользователь не найден, функция возвращает ошибку в виде Err с соответствующим сообщением. */
pub fn get_password_hash_from_username(name: String) -> Result<String, std::io::Error> {
    let connection = establish_connection(); //Устанавливает соединение с базой данных.

    let mut stmt = connection
        .prepare("SELECT password FROM users WHERE username = ? LIMIT 1") //Подготавливает SQL-запрос для выбора пароля из таблицы пользователей (users) по имени пользователя (username). Ожидает получить не более одной строки (LIMIT 1).
        .expect("не удалось выбрать значения fn get_password_hash_from_username"); //обработка ошибки
    let password: Option<String> = stmt
        .query_row(params![name], |row| row.get(0)) //Выполняет SQL-запрос, передавая имя пользователя (name) в параметрах.
        .optional() //Возвращает опциональное значение строки, представляющее пароль пользователя.
        .expect("пароль не найден fn get_password_hash_from_username");

    match password {
        //Использует сопоставление с образцом для проверки результата запроса.
        Some(pass) => Ok(pass), //Если пароль найден, возвращает Ok с паролем
        None => Err(std::io::Error::new(
            //иначе возвращает Err с сообщением об отсутствии пользователя.
            std::io::ErrorKind::Other,
            "no user found",
        )),
    }
}

/* Функция выполняет запрос к базе данных для получения ID пользователя по его имени (username).
Если запрос успешен, она возвращает полученный ID. В случае возникновения ошибки, она выводит
сообщение об ошибке и возвращает значение 0. */
pub fn get_id_users_table(username: String) -> i64 {
    let conn = establish_connection(); //Создаем соединение с базой данных.
    let mut stmt = conn
        .prepare("SELECT id FROM users WHERE username = ?") //Подготавливаем SQL-запрос для выбора ID пользователя по его имени. Знак вопроса (?) является плейсхолдером, который будет заменен значением параметра при выполнении запроса.
        .expect("не удалось выбрать id организатора");
    stmt.query_row(params![username], |row| row.get(0)) // Выполняем запрос, используя подготовленный запрос stmt и передаем параметр username. Функция query_row возвращает объект, который можно использовать для извлечения данных. Лямбда-выражение |row| row.get(0) извлекает значение из первого столбца результата запроса, представляющее ID пользователя.
        .unwrap_or_else(|err| {
            //Обрабатываем результат запроса. Если запрос завершается успешно, возвращаем полученный ID. В противном случае (возникла ошибка), выводим сообщение об ошибке и возвращаем значение 0 (или другое, на ваш выбор) как fallback.
            eprintln!(
                "Не удалось получить id пользователя из базы данных: {}",
                err
            );
            0 // Или какое-то другое значение по умолчанию, если нужно
        })
}

pub fn create_new_user(
    conn: &Connection,
    username_new_user: String,
    password_new_user: String,
    city_new_user: String,
    role_new_user: String,
    first_name_user: String,
    patronymic_user: String,
    last_name_user: String,
    phone_user: String,
    mail_user: String,
) -> Result<UsersTable, Error> {
    let new_user = NewUser {
        username_new_user,
        password_new_user,
        city_new_user,
        role_new_user,
    };

    //проверка на существование юзера с таким логином
    let username_exist: bool = match conn.query_row(
        "SELECT EXISTS(SELECT username FROM users WHERE username = ?)",
        params![&new_user.username_new_user],
        |row| row.get(0),
    ) {
        Ok(result) => result,
        Err(Error::QueryReturnedNoRows) => false,
        Err(err) => panic!(
            "Ошибка при проверке наличия login при регистрации пользователя: {}",
            err
        ),
    };

    if username_exist {
        return Err(Error::QueryReturnedNoRows);
    }

    conn.execute(
        "INSERT INTO users (username,
        password,
        city,
        role,
        first_name_user,
        patronymic_user,
        last_name_user,
        phone_user,
        mail_user) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            &new_user.username_new_user,
            &new_user.password_new_user,
            &new_user.city_new_user,
            &new_user.role_new_user,
            first_name_user,
            patronymic_user,
            last_name_user,
            phone_user,
            mail_user
        ],
    )
    .expect(
        "не удалось вставить данные user_NewUser, password_NewUser, city_NewUser, role_NewUser",
    );

    //после записи в базу данных нужно провести сортировку по убыванию по id и извлечь самую первую запись
    let mut stmt = conn
        .prepare(
            "SELECT id, \
    username, password, city, role FROM users ORDER BY id DESC",
        )
        .expect("не удалось произвести сортировку юзеров по убыванию id");

    let user_new_user_result = stmt
        .query_map([], |row| {
            Ok(UsersTable {
                id_table: row.get(0)?,
                username_table: row.get(1)?,
                password_table: row.get(2)?,
                city_table: row.get(3)?,
                role_table: row.get(4)?,
            })
        })
        .expect("проверь let user_new_user_result");

    let x = user_new_user_result
        .map(|mapped_rows| mapped_rows.unwrap())
        .next()
        .unwrap();

    Ok(x) // Получение результата создания нового юзера
}

/* #[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<User, Self::Error> {
        let user = request //получаем куки по имени session_token
            .cookies() //извлекаем все куки из запроса
            .get_private("session_token") //ищем конкретную приватную куку с именем "session_token"
            .map(|cookie| { //получение куки продолжается до этой строки

                let token_from_cookie = cookie.value().to_string(); // Получение значения токена из куки

                let connection = establish_connection(); //подключаемся к бд

                let mut stmt = connection
                    .prepare("SELECT token FROM users_session WHERE token = ? ORDER BY id DESC LIMIT 1") //обращаемся к таблице user_session, первый token-название столбца в котором идёт поиск, WHERE token = ? LIMIT 1, где ? - это плейсхолдер для значения, которое будет использовано при выполнении подготовленного запроса, ищем только одну строку в таблице, ORDER BY id DESC - сортировка по id по убыванию
                    .expect("токен не найден fn from_request(request: &'a Request<'r>)"); //обработка ошибки
                let user_id_result_users_session = stmt
                    .query_row(params![&token_from_cookie], |row| { //подставляем в плэйсхолдер токен из приватной куки
                        let user_id_users_session: Result<i64, _> = row.get(1); //получаем из найденной строки user_id
                        Ok(user_id_users_session.map(|_| 1).unwrap_or(0)) //если в таблице users_session user_id найден то user_id_result=1, если не найден, то 0

                    });

                if user_id_result_users_session.is_ok() && user_id_result_users_session.unwrap() == 1 { //условие означает, что если user_id_result=1 то выполняем код дальше
                    let user_id_result_users_session = stmt
                        .query_row(params![&token_from_cookie], |row| {
                            row.get(1)
                        })
                        .expect("ошибка получения user_id в if user_id_result_users_session.is_ok() && user_id_result_users_session.unwrap() == 1 {");

                    let mut stmt = connection
                        .prepare("SELECT id FROM users WHERE user_id = ? LIMIT 1")
                        .expect("юзер не найден if user_id_result_users_session.is_ok() && user_id_result_users_session.unwrap() == 1 {");
                    let username_result_users = stmt
                        .query_row(params![user_id_result_users_session], |row| {
                            let username: Result<String, _> = row.get(1);
                            Ok(username.map(|_| 1).unwrap_or(0)) //если в таблице users user найден то user_result=1, если не найден, то 0
                        });

                    if username_result_users.is_ok() && username_result_users.unwrap() == 1 {
                        let username_result_users: String = stmt
                            .query_row(params![&user_id_result_users_session], |row| {
                                row.get(1)
                            })
                            .expect("ошибка получения user в if user_result_users.is_ok() && user_result_users.unwrap() == 1 {");

                        return Some(User {
                            id: user_id_result_users_session,
                            username: username_result_users.clone(),
                        });
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }

            });
        match user {
            Some(uid) => match uid {
                Some(user) => {
                    return Outcome::Success(user);
                }
                None => return Outcome::Forward(Status::Unauthorized),
            },
            None => return Outcome::Forward(Status::Unauthorized),
        }
    }
} */

//создание новой сессии юзера
pub fn create_new_user_session(
    conn: &Connection,
    user_id: i64,
    token: String,
) -> Result<UsersSession, Error> {
    let new_user_session = NewUserSession {
        user_id_new_user_session: user_id,
        token_new_user_session: token,
    };

    conn.execute(
        "INSERT INTO users_sessions (user_id, token) VALUES (?, ?)",
        params![
            &new_user_session.user_id_new_user_session,
            &new_user_session.token_new_user_session
        ],
    )?; //.expect("не удалось вставить данные user_id_NewUserSession, token_NewUserSession");

    //после записи в базу данных нужно провести сортировку по убыванию по id и извлечь самую первую запись
    let mut stmt = conn
        .prepare(
            "SELECT id, \
    user_id, token FROM users_sessions ORDER BY id DESC LIMIT 1",
        ) //.expect("не удалось выбрать последнюю юзер_сессию");
        .map_err(|err| {
            eprintln!(
                "Не удалось подготовить запрос для выбора сеанса пользователя: {}",
                err
            );
            err
        })?;

    let user_session_result = stmt
        .query_map(params![], |row| {
            Ok(UsersSession {
                id_user_session: row.get(0)?,
                user_id_users_session: row.get(1)?,
                token_users_session: row.get(2)?,
            })
        }) //.expect("проверь let user_session_result");
        .map_err(|err| {
            eprintln!("Не удалось получить данные из таблицы user_session {}", err);
            err
        })?;

    let x = user_session_result
        .map(|mapped_rows| mapped_rows.unwrap())
        .next()
        .unwrap_or_else(|| {
            eprintln!("No user session data found");
            UsersSession {
                id_user_session: 0,
                user_id_users_session: 0,
                token_users_session: String::new(),
            }
        });

    Ok(x) // Получение результата
}

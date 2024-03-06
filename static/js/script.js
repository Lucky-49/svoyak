var STATE = {
  connected: false,
};

// Функция для установки статуса соединения.
function setConnectedStatus(status) {
  STATE.connected = status;
  var statusDiv = document.getElementById('status');
  statusDiv.className = status ? 'connected' : 'reconnecting';
}

// Функция для подписки на сервер-отправителя событий.
function subscribe(uri) {
  var retryTime = 1;

function connect(uri) {
    const events = new EventSource(uri);

      events.addEventListener("open", () => {
      setConnectedStatus(true);
      console.log(`connected to event stream at ${uri}`);
      retryTime = 1;
    });

    events.addEventListener("error", () => {
      setConnectedStatus(false);
      events.close();

      let timeout = retryTime;
      retryTime = Math.min(64, retryTime * 2);
      console.log(`connection lost. attempting to reconnect in ${timeout}s`);
      setTimeout(() => connect(uri), (() => timeout * 1000)());
    });
  }

  connect(uri);
}

// Обработчик отправки формы для нового сообщения
function handleNewMessageFormSubmit(e) {
  e.preventDefault();

  const messageField = document.getElementById("message"); // Получить элемент с id "message"
  const message = messageField.value;
  const data = document.getElementById("datepicker").value; // Получить значение даты
  const time = document.getElementById("timepicker").value; // Получить значение времени


  if (!message || !data || !time) return;

  if (STATE.connected) {
    // Отправить данные на сервер
    fetch("/message", {
      method: "POST",
      body: new URLSearchParams({ message, data, time }),
    }).then((response) => {
      if (response.ok) {
        messageField.value = "";
        document.getElementById("datepicker").value = ""; // Сбросить значение даты
        document.getElementById("timepicker").value = ""; // Сбросить значение времени

        // После успешной аутентификации, перенаправьте пользователя на главную страницу
        if (isAuthenticated) {
          window.location.href = "/index.html";
        }
      }
    });
  }
}

// Инициализация соединения.
function init() {
  subscribe('/events');
  const loginForm = document.getElementById("login-form");
  loginForm.addEventListener("submit", handleLoginFormSubmit);
}

init();

// Обработчик отправки формы для страницы входа
function handleLoginFormSubmit(e) {
  e.preventDefault();

  const usernameField = document.getElementById("username");
  const passwordField = document.getElementById("password");

  // Здесь вы можете отправить запрос на сервер для проверки логина и пароля
  // Если вход успешен, перенаправьте пользователя на index.html
  if (usernameField.value === "ваш_логин" && passwordField.value === "ваш_пароль") {
    window.location.href = "/index.html";
  }
}

function validation() {
  let location_game = document.getElementById("announce_message").value.trim(); //поле места проведения игры
  let  price_player = document.getElementById("price_player").value.trim(); //цена игрока
  let price_spectator = document.getElementById("price_spectator").value.trim(); //цена зрителя
  let seats_spectator = document.getElementById("seats_spectator").value.trim();

  if (location_game !== "" &&
      seats_spectator !== "" &&
  price_player !== "" &&
  price_spectator !== "") { //если хоть в одном из полей пусто, то конпка
    document.getElementById("announce_button").disabled = false; //не активна
  } else {
    document.getElementById("announce_button").disabled = true; //активна
  }
}
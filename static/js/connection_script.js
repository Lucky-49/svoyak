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
      }
    });
  }
}

// Инициализация соединения.
function init() {
  subscribe('/events');
   const newMessageForm = document.getElementById("new-message");
  newMessageForm.addEventListener("submit", handleNewMessageFormSubmit); // Добавить обработчик
}

init();
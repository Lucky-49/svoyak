function validation() {
    let pacname = document.getElementById("pac_name").value.trim(); //id поля названия пакета вопросов

    if (pacname !== "") { //если в поле названия темы пакета пусто, то конпка
        document.getElementById("create_questions_pac").disabled = false; //не активна
    } else {
        document.getElementById("create_questions_pac").disabled = true; //активна
    }
}
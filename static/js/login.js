function validation() {
    let username = document.getElementById("username").value;
    let pass = document.getElementById("password").value;
    if (username !== "" && pass !== "") { //если в полях юзернейм или пасс пусто, то конпка
        document.getElementById("submit").disabled = false; //не активна
    } else {
        document.getElementById("submit").disabled = true; //активна
    }
}
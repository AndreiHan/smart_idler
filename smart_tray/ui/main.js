const { invoke } = window.__TAURI__.tauri;

//---Utils

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

//---Table refresh

let refresh_table_list = [
  ["robot-inputs", "tauri_get_db_count"],
  ["current-interval", "get_data", "force_interval"],
  ["last-input", "get_data", "robot_input"],
];

function refresh_stats_table() {
  refresh_table_list.forEach(refresh_table_element);
}

async function refresh_table_element(table_element) {
  document.getElementById(table_element[0]).innerText = "Loading...";
  await sleep(250);
  if (table_element[2] != undefined) {
    invoke(table_element[1], { data: table_element[2] }).then(
      (input) => (document.getElementById(table_element[0]).innerText = input)
    );
  } else {
    invoke(table_element[1]).then(
      (input) => (document.getElementById(table_element[0]).innerText = input)
    );
  }
}

//---Checkbox refresh

let refresh_checkbox_list = [
  ["log-toggle", "logging"],
  ["mnts-toggle", "maintenance"],
  ["startup-toggle", "startup"],
];

function refresh_all_checkboxes() {
  refresh_checkbox_list.forEach(refresh_checkbox);
}

async function refresh_checkbox(checkbox_element) {
  document.getElementById(checkbox_element[0]).checked = false;
  await sleep(250);
  invoke("get_state", { data: checkbox_element[1] }).then(
    (input) => (document.getElementById(checkbox_element[0]).checked = input)
  );
}

//---Event Listeners

//-On Load
window.addEventListener("DOMContentLoaded", () => {
  refresh_stats_table();
  refresh_all_checkboxes();
});

//-Buttons
let submit_btn = document.getElementById("refresh-btn");
submit_btn.addEventListener("click", () => {
  refresh_stats_table();
  refresh_all_checkboxes();
});


//-Auto shutdown
let time_input = document.getElementById("timed-input");
time_input.addEventListener("change", () => {
  var timeValue = event.target.value;
  console.log("Time value: ", timeValue);
  invoke("set_shutdown", {hour: timeValue});
  document.getElementById("timed-stop").checked = true;
});

//-Enable shutdown
let shutdown_checkbox = document.getElementById("timed-stop");
shutdown_checkbox.addEventListener("click", () => {
  if (shutdown_checkbox.checked == true) {
    var timeValue = document.getElementById("timed-input").value;
    invoke("set_shutdown", {hour: timeValue});
  } else {
    invoke("set_shutdown", {hour: "STOP"});
  }
});

//-Submit button
let interval_btn = document.getElementById("submit-interval-btn");
interval_btn.addEventListener("click", async () => {
  let textbox = document.getElementById("interval-data");
  if (isNaN(textbox.value) || textbox.value < 60) {
    textbox.placeholder = "Invalid data";
    textbox.value = "";
  } else {
    invoke("set_force_interval", {
      interval: textbox.value,
    });
    textbox.placeholder = "SUCCESSFUL";
    textbox.value = "";
    refresh_stats_table();
  }
});

//-Checkboxes

let settings_checkbox_list = [
  ["log-toggle", "logging"],
  ["mnts-toggle", "maintenance"],
  ["startup-toggle", "startup"],
];

settings_checkbox_list.forEach(set_event_listener);

function set_event_listener(checkbox) {
  let startup_checkbox = document.getElementById(checkbox[0]);
  startup_checkbox.addEventListener("click", () => {
    if (document.getElementById(checkbox[0]).checked == true) {
      invoke("set_registry_state", { data: checkbox[1], wanted_status: true });
    } else {
      invoke("set_registry_state", { data: checkbox[1], wanted_status: false });
    }
  });
}

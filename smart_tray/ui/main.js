const { invoke } = window.__TAURI__.tauri;

//---Utils
// eslint-disable-next-line i18n-text/no-en
const INVALID_DATA_MESSAGE = "Invalid data";
const LOADING_TEXT = "Loading...";
const DEFAULT_SHUTDOWN_TIME = "19:00";
const STOP_TIME = "STOP";
const SUCCESSFUL_MESSAGE = "SUCCESSFUL";
const SHUTDOWN_CLOCK_ID = "get_shutdown_clock";
const SHUTDOWN_STATE_ID = "get_shutdown_state";
const SET_SHUTDOWN_ID = "set_shutdown";
const SET_FORCE_INTERVAL_ID = "set_force_interval";
const SET_REGISTRY_STATE_ID = "set_registry_state";
const GET_STATE_ID = "get_state";

//---Default values
const DEFAULT_MINIMUM_INTERVAL = 60;

const CHECKBOX_LIST = [
  ["log-toggle", "logging"],
  ["mnts-toggle", "maintenance"],
  ["startup-toggle", "startup"],
];

const CHECKBOX_SETTINGS_LIST = [
  ["log-toggle", "logging"],
  ["mnts-toggle", "maintenance"],
  ["startup-toggle", "startup"],
];

const DOM_ELEMENTS = {
  clockValue: document.getElementById("timed-input"),
  clockStatus: document.getElementById("timed-stop"),
  intervalData: document.getElementById("interval-data"),
  submitIntervalBtn: document.getElementById("submit-interval-btn"),
};

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

//---Table refresh

const refreshTableList = [
  ["robot-inputs", "tauri_get_db_count"],
  ["current-interval", "get_data", "force_interval"],
  ["last-input", "get_data", "robot_input"],
];

function refreshStatsTable() {
  for (const tableElement of refreshTableList) {
    refreshTableElement(tableElement);
  }
}

async function refreshTableElement(tableElement) {
  document.getElementById(tableElement[0]).innerText = LOADING_TEXT;
  await sleep(250);
  let input;
  if (tableElement[2] !== undefined) {
    input = await invoke(tableElement[1], { data: tableElement[2] });
  } else {
    input = await invoke(tableElement[1]);
  }
  document.getElementById(tableElement[0]).innerText = input;
}

//---Checkbox refresh

function refreshAllCheckboxes() {
  for (const checkboxElement of CHECKBOX_LIST) {
    refreshCheckbox(checkboxElement);
  }
}

async function refreshCheckbox(checkboxElement) {
  document.getElementById(checkboxElement[0]).checked = false;
  await sleep(250);
  const input = await invoke(GET_STATE_ID, { data: checkboxElement[1] });
  document.getElementById(checkboxElement[0]).checked = input;
}

//---Event Listeners

//-On Load
window.addEventListener("DOMContentLoaded", async () => {
  refreshStatsTable();
  refreshAllCheckboxes();

  //-Shutdown load
  DOM_ELEMENTS.clockValue.value = await invoke(SHUTDOWN_CLOCK_ID, {});
  DOM_ELEMENTS.clockStatus.checked = await invoke(SHUTDOWN_STATE_ID, {});
});

//-Buttons

//-Auto shutdown
DOM_ELEMENTS.clockValue.addEventListener("change", () => {
  const timeValue = DOM_ELEMENTS.clockValue.value;
  invoke(SET_SHUTDOWN_ID, { hour: timeValue });
  DOM_ELEMENTS.clockStatus.checked = true;
});

//-Enable shutdown
DOM_ELEMENTS.clockStatus.addEventListener("click", () => {
  if (DOM_ELEMENTS.clockStatus.checked) {
    let timeValue = DOM_ELEMENTS.clockValue.value;
    if (timeValue === STOP_TIME || timeValue === "") {
      timeValue = DEFAULT_SHUTDOWN_TIME;
    }
    invoke(SET_SHUTDOWN_ID, { hour: timeValue });
    DOM_ELEMENTS.clockValue.value = timeValue;
  } else {
    invoke(SET_SHUTDOWN_ID, { hour: STOP_TIME });
    DOM_ELEMENTS.clockValue.value = STOP_TIME;
  }
});

//-Submit button
DOM_ELEMENTS.submitIntervalBtn.addEventListener("click", async () => {
  const textbox = DOM_ELEMENTS.intervalData;
  if (isNaN(textbox.value) || textbox.value < DEFAULT_MINIMUM_INTERVAL) {
    textbox.placeholder = INVALID_DATA_MESSAGE;
    textbox.value = "";
  } else {
    invoke(SET_FORCE_INTERVAL_ID, {
      interval: textbox.value,
    });
    textbox.placeholder = SUCCESSFUL_MESSAGE;
    textbox.value = "";
    refreshStatsTable();
  }
});

//-Checkboxes

for (const checkbox of CHECKBOX_SETTINGS_LIST) {
  setEventListener(checkbox);
}

function setEventListener(checkbox) {
  const startupCheckbox = document.getElementById(checkbox[0]);
  startupCheckbox.addEventListener("click", () => {
    if (document.getElementById(checkbox[0]).checked) {
      // eslint-disable-next-line camelcase
      invoke(SET_REGISTRY_STATE_ID, { data: checkbox[1], wanted_status: true });
    } else {
      invoke(SET_REGISTRY_STATE_ID, {
        data: checkbox[1],
        // eslint-disable-next-line camelcase
        wanted_status: false,
      });
    }
  });
}

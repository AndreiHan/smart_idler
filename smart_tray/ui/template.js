const { invoke } = window.__TAURI__.tauri;

//---Utils
// eslint-disable-next-line i18n-text/no-en
const INVALID_DATA_MESSAGE = "Invalid data";
const LOADING_TEXT = "Loading...";
const DEFAULT_SHUTDOWN_TIME = "19:00";
const STOP_TIME = "STOP";
const SUCCESSFUL_MESSAGE = "SUCCESSFUL";
const SHUTDOWN_CLOCK_ID = "plugin:general|get_shutdown_clock";
const SHUTDOWN_STATE_ID = "plugin:general|get_shutdown_state";
const SET_SHUTDOWN_ID = "plugin:general|set_shutdown";
const SET_FORCE_INTERVAL_ID = "plugin:general|set_force_interval";
const SET_REGISTRY_STATE_ID = "plugin:general|set_registry_state";
const GET_STATE_ID = "plugin:general|get_state";

//---Default values
const DEFAULT_MINIMUM_INTERVAL = 60;

const DOM_ELEMENTS = {
  clockValue: document.getElementById("timed-input"),
  clockStatus: document.getElementById("timed-stop"),
  intervalData: document.getElementById("interval-data"),
  submitIntervalBtn: document.getElementById("submit-interval-btn"),
};

//---Table refresh

const refreshTableList = [
  ["current-interval", "plugin:general|get_data", "force_interval"],
  ["last-input", "plugin:general|get_data", "robot_input"],
];

function refreshStatsTable() {
  Promise.all(refreshTableList.map(refreshTableElement));
}

function refreshTableElement(tableElement) {
  document.getElementById(tableElement[0]).innerText = LOADING_TEXT;
  let inputPromise;
  if (tableElement[2] !== undefined) {
    inputPromise = invoke(tableElement[1], { data: tableElement[2] });
  } else {
    inputPromise = invoke(tableElement[1]);
  }
  // eslint-disable-next-line github/no-then
  inputPromise.then((input) => {
    document.getElementById(tableElement[0]).innerText = input;
  });
}

//---Event Listeners

//-On Load
window.addEventListener("DOMContentLoaded", () => {
  refreshStatsTable();

  //-Shutdown load
  // eslint-disable-next-line github/no-then
  invoke(SHUTDOWN_CLOCK_ID, {}).then(
    (value) => (DOM_ELEMENTS.clockValue.value = value),
  );
  // eslint-disable-next-line github/no-then
  invoke(SHUTDOWN_STATE_ID, {}).then(
    (checked) => (DOM_ELEMENTS.clockStatus.checked = checked),
  );
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

async function update_interval() {
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
}

//-Submit button
DOM_ELEMENTS.submitIntervalBtn.addEventListener("click", update_interval);

//-Data Field
DOM_ELEMENTS.intervalData.addEventListener("keypress", async (event) => {
  if (event.key === "Enter") {
    event.preventDefault();
    DOM_ELEMENTS.submitIntervalBtn.click();
  }
});

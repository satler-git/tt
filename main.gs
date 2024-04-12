// Tent API Backend
// Trigger: onFormSubmit -> onSubmit(e)

function removeNullsFromArray(arr) {
  if (!Array.isArray(arr)) {
    throw new Error('Input is not an array.');
  }

  return arr.filter(element => element !== null && element !== undefined);
}

function getJsonMusicQueue() {
  const ss = SpreadsheetApp.getActiveSpreadsheet();
  const sheet = ss.getSheets()[0];
  const data = sheet.getRange(1, 1, sheet.getLastRow(), sheet.getLastColumn()).getValues();

  const objectArray = [];
  for (let i = 1; i < data.length; i++) {
    const item = {};
    const headerlist = ["time_stamp", "mail", "song_name", "artist_name", "uuid"]

    for (let j = 0; j < data[0].length; j++) {
      item[headerlist[j]] = data[i][j];
    }
    objectArray.push(item);
  }

  const modifiedJson = {
    contents: removeNullsFromArray(objectArray)
  };
  const json = JSON.stringify(modifiedJson);

  return json;
}

const doGet = (e) => {
  return ContentService.createTextOutput(getJsonMusicQueue());
}

function onSubmit(e) {
  const ss = SpreadsheetApp.getActiveSpreadsheet();
  const sheet = ss.getSheets()[0];

  sheet.getRange(e.range.getRow(), 5).setValue(Utilities.getUuid());
}
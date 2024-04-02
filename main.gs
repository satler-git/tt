// Tent API Backend

function removeNullsFromArray(arr) {
  if (!Array.isArray(arr)) {
    throw new Error('Input is not an array.');
  }

  return arr.filter(element => element !== null && element !== undefined);
}

function getJsonMusicQueue(acquired = 0) {
  const ss = SpreadsheetApp.getActiveSpreadsheet();
  const sheet = ss.getSheets()[0];
  const data = sheet.getRange(1, 1, sheet.getLastRow(), sheet.getLastColumn()).getValues();

  const objectArray = [];
  for (let i = 1 + acquired; i < data.length; i++) {
    const item = {};
    const headerlist = ["time_stamp", "mail", "song_name", "artist_name"]

    for (let j = 0; j < data[0].length; j++) {
      item[headerlist[j]] = data[i][j];
    }
    objectArray.push(item);
  }

  const modifiedJson = removeNullsFromArray(objectArray);
  const json = JSON.stringify(modifiedJson);

  return json;
}

const doGet = (e) => {
  const acquired = parseInt(e.parameter.acquired) || 0;

  return ContentService.createTextOutput(getJsonMusicQueue(acquired));
}

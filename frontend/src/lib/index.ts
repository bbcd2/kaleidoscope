export enum WebsocketServerMessage {
    "ClientHello",
}

/** Duration units -- 60**unit */
export enum DurationUnit {
    Seconds = 0,
    Minutes = 1,
    Hours = 2,
}

/** Recording status from the server */
export enum Stage {
    "Waiting in Queue" = 0,
    // OK statuses -- fixme: complete bs: needed to deploy
    "Initialising" = 1,
    "Downloading" = 2,
    "Combining" = 3,
    "Encoding" = 4,
    "Uploading Result" = 5,
    "Completed" = 6,
    /** Separation between OK statuses and error statuses */
    "_SENTINEL_MAX_OK" = 7,
    // Errors
    "Downloading Failed" = 10,
    "Encoding Failed" = 11,
    "Uploading Failed" = 12,
}

// prettier-ignore
/** Sources with groups */
export const SOURCES: { [key: string]: string[] } = {
    'BBC NEWS': [
        'BBC NEWS CHANNEL HD',
        'BBC WORLD NEWS AMERICA HD'
    ],
    'BBC ONE': [
        'BBC ONE HD',
        'BBC ONE WALES HD',
        'BBC ONE SCOTLAND HD',
        'BBC ONE NORTHERN IRELAND HD',
        'BBC ONE CHANNEL ISLANDS HD',
        'BBC ONE EAST HD',
        'BBC ONE EAST MIDLANDS HD',
        'BBC ONE EAST YORKSHIRE & LINCONSHIRE HD',
        'BBC ONE LONDON HD',
        'BBC ONE NORTH EAST HD',
        'BBC ONE NORTH WEST HD',
        'BBC ONE SOUTH HD',
        'BBC ONE SOUTH EAST HD',
        'BBC ONE SOUTH WEST HD',
        'BBC ONE WEST HD',
        'BBC ONE WEST MIDLANDS HD',
        'BBC ONE YORKSHIRE HD',
    ],
    'BBC TWO': [
        'BBC TWO HD',
        'BBC TWO NORTHERN IRELAND HD',
        'BBC TWO WALES DIGITAL',
    ],
    OTHER: [
        'BBC THREE HD',
        'BBC FOUR HD',
        'CBBC HD',
        'CBEEBIES HD',
        'BBC SCOTLAND HD',
        'BBC PARLIAMENT',
        'BBC ALBA',
        'S4C',
    ]
};

/** Find a source's name by its ID */
export const lookupSourceById = (id: number): string => {
    return Object.values(SOURCES).flat(1)[id];
};

/** Get the maximum day for a given month (between 1-12) */
export const getMaxDay = (year: number, month: number): number => {
    // prettier-ignore
    switch (month) {
        case 2: return year % 4 == 0 && year % 100 != 0 ? 29 : 28;
        case 4: return 30;
        case 6: return 30;
        case 9: return 30;
        case 11: return 30;
        default: return 31;
    }
};

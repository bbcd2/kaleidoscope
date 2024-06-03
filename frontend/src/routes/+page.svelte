<script lang="ts">
    import moment from "moment-timezone";
    import { onMount } from "svelte";
    import { enhance } from "$app/forms";

    import { DurationUnit, SOURCES, Status, lookupSourceById, getMaxDay } from "$lib";

    import {
        Label,
        Input,
        Select,
        Button,
        Toast,
        Modal,
        Video,
        Progressbar,
        Checkbox,
    } from "flowbite-svelte";
    import {
        ArrowRightOutline,
        LinkOutline,
        PaperPlaneOutline,
        BullhornSolid,
    } from "flowbite-svelte-icons";
    import Error from "./+error.svelte";

    const [flashSnackbar, stopFlashSnackbar] = (function () {
        // fixme: use signals and NOT dom manipulation in javascript framework
        // fixme: (Svelte Esq. is very angry with its virtual dom changing!)
        // fixme: For Fuck Sake, you poriferas (/lh /t)
        let snackbar = null;
        try {
            snackbar = document.querySelector("#snackbar");
        } catch (e) {}
        return [
            /** Flash a message to the snackbar */
            (message: string, error: boolean) => {
                console.debug({ message, error });
            },
            /** Stop flashing a message to the snackbar */
            () => {
                if (snackbar) snackbar.className = "";
            },
            /** (the "broken" variant -- keeps crashing svelte lmao) */
            (message: string, error: boolean) => {
                const snackbar = document.querySelector("#snackbar");
                if (!snackbar) return;
                snackbar.className = "snackbar active " + error ? "primary" : "error";
                snackbar.innerHTML = message;
                setTimeout(stopFlashSnackbar, 3_000);
            },
        ];
    })();

    /** Handle a websocket message */
    const handleWsMessage = ({ data }: { data: string }) => {
        console.debug(`websocket message: ${data}`);
        let response;
        try {
            response = JSON.parse(data);
        } catch (e) {
            return flashSnackbar(`Failed to parse server response: ${data}`, true);
        }
    };

    let ws: null | WebSocket = null;
    let wsConnected: boolean = false;
    onMount(() => {
        // Connect to backend websocket
        flashSnackbar("Connecting to the server", false);
        console.log("Help");
        ws = new WebSocket("ws://localhost:8081/websocket"); // fixme: debug url
        ws.onopen = () => {
            wsConnected = true;
            stopFlashSnackbar();
        };
        ws.onmessage = handleWsMessage;
        ws.onclose = ws.onerror = () =>
            flashSnackbar(
                !wsConnected
                    ? `Failed to connect to the server.`
                    : `Connection to server unexpectedly closed!>` +
                          ' Check <a href="status.bbcd.uk.to">the status page</a>!',
                true,
            );
    });

    // Form and database
    interface RecordingsInfo {
        obtained: boolean;
        error: string | undefined;
        recordings: any[] | undefined;
    }
    let recordings: RecordingsInfo = { obtained: false, error: undefined, recordings: undefined };
    let currentJobs: string[] = [];

    let showRecordingModals: { [key: string]: boolean } = {};

    // Inputs
    let encode = true;
    // Timezone should be in UK time (BST or GMT)
    const currentDate = new Date();
    let [month, day] = [currentDate.getMonth() + 1, currentDate.getDate()].map((x) => `${x}`);
    let maxDay = getMaxDay(currentDate.getUTCFullYear(), parseInt(month));
    $: {
        maxDay = getMaxDay(currentDate.getUTCFullYear(), parseInt(month));
        day = `${Math.min(maxDay, parseInt(day))}`;
    }
    let [startHour, startMinute, startSecond] = [
        currentDate.getHours(),
        currentDate.getMinutes(),
        currentDate.getSeconds(),
    ].map((x) => {
        // We need to store these as strings for native input elements
        return `${x}`;
    });
    let durationUnit = DurationUnit.Minutes;
    let duration = 0;
    // Precision of seconds, i.e. *1000 when converting to a `Date`
    let [startTimestamp, endTimestamp] = [0, 0];
    // Get POSIX timestamp, accounting for UK timezone
    $: {
        const totalSeconds = duration * 60 ** (durationUnit as number);
        startTimestamp = moment
            .tz(
                [
                    currentDate.getUTCFullYear(),
                    parseInt(month) - 1,
                    parseInt(day),
                    parseInt(startHour),
                    parseInt(startMinute),
                    parseInt(startSecond),
                ],
                "Europe/London",
            )
            .unix();
        endTimestamp = startTimestamp + totalSeconds;
    }
    let channel = Object.values(SOURCES)[0][0];
</script>

<!-- fixme: uh yeah i got rid of everything -->

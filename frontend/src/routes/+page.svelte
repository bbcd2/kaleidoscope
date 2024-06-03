<script lang="ts">
    import moment from "moment-timezone";
    import { onMount } from "svelte";
    import axios from "axios";

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

    import { DurationUnit, SOURCES, getMaxDay, lookupSourceById, Stage } from "$lib";

    /** Flash a message to the snackbar */
    const [flashSnackbar, stopFlashSnackbar] = (function () {
        let snackbar = null;
        try {
            snackbar = document.querySelector("#snackbar");
        } catch (e) {}
        return [
            /** Flash a message to the snackbar */
            (message: string, error: boolean) => {
                const snackbar = document.querySelector("#snackbar");
                if (!snackbar) return;
                snackbar.className = "snackbar active " + error ? "primary" : "error";
                snackbar.innerHTML = message;
                setTimeout(stopFlashSnackbar, 3_000);
            },
            /** Stop flashing a message to the snackbar */
            () => {
                if (snackbar) snackbar.className = "";
            },
        ];
    })();

    /** Get some recordings */
    const getRecordings = async () => {
        const response = await axios.get(
            `http://localhost:8081/list-recordings?start=0&count=15` /* fixme */,
        );
        if (response.status !== 200)
            recordings = { obtained: false, error: `${response.data}`, recordings: undefined };
        else
            recordings = {
                obtained: true,
                error: undefined,
                recordings: response.data,
            };
    };

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
        ws = new WebSocket("ws://localhost:8081/websocket" /* fixme */);
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
    interface RecordingInfo {
        uuid: string;
        user_id: number;
        rec_start: string; // ISO 8601
        rec_end: string; // ISO 8601
        status: string;
        stage: number;
        channel: number;
    }
    interface RecordingsInfo {
        obtained: boolean;
        error: string | undefined;
        recordings: RecordingInfo[] | undefined;
    }
    let recordings: RecordingsInfo = { obtained: false, error: undefined, recordings: undefined };

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

{#await getRecordings()}
    <p class="italic text-center">Fetching recordings...</p>
{/await}
{#if recordings.error}
    <p>Failed to fetch recordings: {recordings.error}</p>
{/if}
<div class="flex justify-center">
    <table
        class="lg:2-xl:mx-[22rem] md:mx-[8rem] mx-2 my-2 border-black dark:border-white w-full table-auto"
    >
        <tbody class="border-2 border-black divide-y dark:border-white">
            {#each recordings.recordings ?? [] as recording}
                <tr
                    ><td class="p-2 border-2 border-black dark:border-white">
                        {(() => {
                            const startDate = new Date(recording.rec_start);
                            const endDate = new Date(recording.rec_end);
                            const sameDay =
                                startDate.getDate() === endDate.getDate() &&
                                startDate.getMonth() === endDate.getMonth() &&
                                startDate.getFullYear() === endDate.getFullYear();
                            return sameDay
                                ? startDate.toLocaleDateString("en-GB", {
                                      month: "short",
                                      day: "2-digit",
                                      timeZone: "Europe/London",
                                  })
                                : startDate.toLocaleDateString("en-GB", {
                                      month: "short",
                                      day: "2-digit",
                                      timeZone: "Europe/London",
                                  }) +
                                      " - " +
                                      endDate.toLocaleDateString("en-GB", {
                                          month: "short",
                                          day: "2-digit",
                                          timeZone: "Europe/London",
                                      });
                        })()}
                    </td>
                    <td class="p-2 border-2 border-black dark:border-white">
                        {(() => {
                            const startDate = new Date(recording.rec_start);
                            const endDate = new Date(recording.rec_end);
                            return (
                                startDate.toLocaleTimeString("en-GB", {
                                    hour: "2-digit",
                                    minute: "2-digit",
                                    timeZone: "Europe/London",
                                }) +
                                " - " +
                                endDate.toLocaleTimeString("en-GB", {
                                    hour: "2-digit",
                                    minute: "2-digit",
                                    timeZone: "Europe/London",
                                })
                            );
                        })()}
                    </td>
                    <td class="p-2 border-2 border-black dark:border-white">
                        {lookupSourceById(recording.channel)}
                    </td>
                    <td class="p-2 border-2 border-black dark:border-white">
                        {Stage[recording.stage]}
                    </td>
                    <td class="p-2 border-2 border-black dark:border-white">
                        <button
                            type="button"
                            on:click={() => (showRecordingModals[recording.uuid] = true)}
                            class="cursor-pointer"><LinkOutline /></button
                        >
                        <Modal
                            title="Recording {recording.uuid}"
                            bind:open={showRecordingModals[recording.uuid]}
                            autoclose
                            outsideclose
                        >
                            <p>Recorded by: <strong>bbcduser</strong></p>
                            <!-- TODO: Have the username change, but thats not entirely needed right now -->
                            <p>
                                Recorded from: <strong>{lookupSourceById(recording.channel)}</strong
                                >
                            </p>
                            <p>
                                Recording date/time: <strong
                                    >{(() => {
                                        const startDate = new Date(recording.rec_start);
                                        const endDate = new Date(recording.rec_end);
                                        const sameDay =
                                            startDate.getDate() === endDate.getDate() &&
                                            startDate.getMonth() === endDate.getMonth() &&
                                            startDate.getFullYear() === endDate.getFullYear();
                                        return sameDay
                                            ? startDate.toLocaleDateString("en-GB", {
                                                  month: "short",
                                                  day: "2-digit",
                                                  timeZone: "Europe/London",
                                              })
                                            : startDate.toLocaleDateString("en-GB", {
                                                  month: "short",
                                                  day: "2-digit",
                                                  timeZone: "Europe/London",
                                              }) +
                                                  " - " +
                                                  endDate.toLocaleDateString("en-GB", {
                                                      month: "short",
                                                      day: "2-digit",
                                                      timeZone: "Europe/London",
                                                  });
                                    })()} | {(() => {
                                        const startDate = new Date(recording.rec_start);
                                        const endDate = new Date(recording.rec_end);
                                        return (
                                            startDate.toLocaleTimeString("en-GB", {
                                                hour: "2-digit",
                                                minute: "2-digit",
                                                timeZone: "Europe/London",
                                            }) +
                                            " - " +
                                            endDate.toLocaleTimeString("en-GB", {
                                                hour: "2-digit",
                                                minute: "2-digit",
                                                timeZone: "Europe/London",
                                            })
                                        );
                                    })()}</strong
                                >
                            </p>

                            {#if recording.stage != Stage.Completed}
                                <p><strong>{Stage[recording.stage]}</strong>: {recording.status}</p>
                            {/if}

                            <hr />

                            {#if recording.stage < Stage.Completed}
                                <!-- Show progressbar if not complete -->
                                <Progressbar
                                    progress={Math.min(
                                        100,
                                        recording.stage * (100 / (Stage["_SENTINEL_MAX_OK"] - 1)),
                                    )}
                                    color="gray"
                                />
                            {:else if recording.stage == Stage.Completed}
                                <!-- Show video if complete -->
                                <div class="flex flex-col items-center">
                                    <Video
                                        src="https://bbcd.uk.to/video/{recording.uuid}.mp4"
                                        controls
                                        trackSrc="{recording.uuid}.mp4"
                                    />
                                    <a
                                        href="/video/{recording.uuid}.mp4"
                                        download
                                        class="mt-3 text-center font-medium focus-within:ring-4 focus-within:outline-none inline-flex items-center justify-center px-5 py-2.5 text-sm text-gray-900 bg-white border border-gray-300 hover:bg-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:hover:border-gray-600 focus-within:ring-gray-200 dark:focus-within:ring-gray-700 rounded-lg"
                                        >Download</a
                                    >
                                </div>
                            {/if}
                        </Modal>
                    </td>
                </tr>
            {/each}
        </tbody>
    </table>
</div>

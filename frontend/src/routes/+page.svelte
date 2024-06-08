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

    let statusFlash: { message: string; error: boolean } | null = null;
    let statusFlashTimeout: NodeJS.Timeout | null = null;
    $: {
        if (statusFlashTimeout) clearTimeout(statusFlashTimeout);
        if (!statusFlash?.error) statusFlashTimeout = setTimeout(() => (statusFlash = null), 3_000);
    }

    let ws: null | WebSocket = null;
    let wsConnected: boolean = false;
    const connectWs = (retry: boolean = false) => {
        // Connect to backend websocket
        if (!retry) statusFlash = { message: "Connecting to the server", error: false };
        ws = new WebSocket("ws://localhost:8081/websocket" /* fixme */);
        ws.onopen = () => {
            wsConnected = true;
            statusFlash = { message: "Connected!", error: false };
            if (retry) fetchRecordings().then();
        };
        ws.onmessage = handleWsMessage;
        ws.onclose = ws.onerror = () => {
            statusFlash = {
                message:
                    (!retry && !wsConnected
                        ? `Failed to connect to the server.`
                        : `Connection to server unexpectedly closed!`) +
                    ' Check <a href="status.bbcd.uk.to">the status page</a>!',
                error: true,
            };
            setTimeout(() => connectWs(true), 1_000);
        };
    };
    const handleWsMessage = ({ data }: { data: string }) => {
        console.debug(`websocket message: ${data}`);
        let response;
        try {
            response = JSON.parse(data);
        } catch (e) {
            return (statusFlash = {
                message: `Failed to parse server response: ${data}`,
                error: true,
            });
        }
        if (response.DatabaseUpdate) handleWsDatabaseUpdate(response.DatabaseUpdate);
    };
    const handleWsDatabaseUpdate = (recording: RecordingInfo) => {
        const updateIdx = recordings.recordings!.findIndex((rec) => rec.uuid === recording.uuid);
        // Assume that a recording update occurs within pagination
        if (updateIdx === -1) recordings.recordings!.splice(0, 0, recording);
        else recordings.recordings![updateIdx] = recording;
        recordings = recordings;
    };

    onMount(() => {
        connectWs();
        fetchRecordings().then();
    });

    const fetchRecordings = async () => {
        let response = undefined;
        try {
            response = await axios.get(
                `http://localhost:8081/list-recordings?start=0&count=15` /* fixme */,
            );
        } catch (e) {
            response = undefined;
        }
        if (response === undefined || response.status !== 200) {
            recordings = {
                obtained: false,
                error: response ? `${response.data}` : "",
                recordings: undefined,
            };
            setTimeout(fetchRecordings, 1_000);
        } else
            recordings = {
                obtained: true,
                error: undefined,
                recordings: response!.data,
            };
    };

    let submitError: string | undefined;
    const submitForm = async () => {
        const response = await axios.post("http://localhost:8081/clip" /* fixme */, {
            start_timestamp: startTimestamp,
            end_timestamp: endTimestamp,
            channel,
            encode,
        });
        if (response.status !== 200) {
            submitError = response.data;
        }
    };

    // Form and database
    interface RecordingInfo {
        uuid: string;
        user_id: number;
        rec_start: string; // ISO 8601
        rec_end: string; // ISO 8601
        status: string;
        short_status: string;
        stage: number;
        channel: string;
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
        // We need to store these as strings for input elements
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
                    currentDate.getUTCFullYear(), // Europe/London follows UTC+0 during December 31st - January 1st
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

<aside
    class="border-2 p-4 lg:2-xl:mx-[22rem] md:mx-[8rem] mx-2 my-10 border-black dark:border-white lg:grid md:grid lg:grid-cols-3 md:grid-cols-3"
>
    <div class="flex flex-col items-start justify-start">
        <!-- Left aligned -->
        <h1 class="pb-2 text-3xl font-bold">From</h1>
        <div class="flex flex-row">
            <div class="w-20">
                <Label for="first_name" class="mb-2 semibold">Day</Label>
                <Input
                    size="md"
                    min="1"
                    max={maxDay}
                    type="number"
                    bind:value={day}
                    class="bg-white border-black rounded-r-none dark:bg-black dark:border-white"
                />
            </div>
            <div class="w-20">
                <Label for="first_name" class="mb-2 semibold">Month</Label>
                <Input
                    size="md"
                    min="1"
                    max="12"
                    type="number"
                    bind:value={month}
                    class="bg-white border-black rounded-l-none dark:bg-black dark:border-white"
                />
            </div>
        </div>

        <div class="flex items-center mt-2">
            <div class="w-20">
                <Label class="mb-2 semibold">Hour</Label>
                <Input
                    size="md"
                    min="0"
                    max="23"
                    type="number"
                    bind:value={startHour}
                    class="bg-white border-black rounded-r-none dark:bg-black dark:border-white"
                />
            </div>
            <div class="w-20">
                <Label class="mb-2 semibold">Minute</Label>
                <Input
                    size="md"
                    min="0"
                    max="59"
                    type="number"
                    bind:value={startMinute}
                    class="bg-white border-black rounded-l-none rounded-r-none dark:bg-black dark:border-white"
                />
            </div>
            <div class="w-20">
                <Label class="mb-2 semibold">Seconds</Label>
                <Input
                    size="md"
                    min="0"
                    max="59"
                    type="number"
                    bind:value={startSecond}
                    class="bg-white border-black rounded-l-none dark:bg-black dark:border-white"
                />
            </div>
        </div>
        <Select
            size="md"
            class="w-full mt-2 bg-white border-black dark:bg-black dark:border-white"
            bind:value={channel}
            placeholder="Select channel..."
        >
            {#each Object.entries(SOURCES) as [channelGroup, channels]}
                <optgroup label={channelGroup}>
                    {#each channels as channelOption}
                        <option value={channelOption}>{channelOption}</option>
                    {/each}
                </optgroup>
            {/each}
        </Select>
    </div>
    <div class="flex flex-col items-center justify-center py-2">
        <ArrowRightOutline />
    </div>

    <div class="flex flex-col gap-4">
        <div class="flex flex-col">
            <h1 class="self-start pb-2 text-3xl font-bold">Length</h1>
            <div class="flex flex-row">
                <div class="w-20 mt-2">
                    <Input
                        size="md"
                        type="number"
                        bind:value={duration}
                        min="0"
                        class="bg-white border-black rounded-r-none dark:bg-black dark:border-white"
                    />
                </div>
                <div class="mt-2 w-100">
                    <Select
                        size="md"
                        class="w-full font-semibold bg-white border-black rounded-l-none dark:bg-black dark:border-white"
                        items={[
                            { value: 0, name: "Seconds" },
                            { value: 1, name: "Minutes" },
                            { value: 2, name: "Hours" },
                        ]}
                        bind:value={durationUnit}
                    />
                </div>
            </div>
            <h6 class="self-start pt-4 font-semibold text-md">
                Ends at {new Date(endTimestamp * 1000).toLocaleString("en-GB", {
                    timeZone: "Europe/London",
                    minute: "numeric",
                    hour: "numeric",
                    second: "numeric",
                    hour12: false,
                })} (UK time)
            </h6>
        </div>
        <div class="flex flex-col">
            <h1 class="self-start pb-2 text-3xl font-bold">Options</h1>
            <div class="flex flex-row items-center gap-1">
                <Checkbox
                    type="checkbox"
                    name="encode"
                    id="encode"
                    checked={encode}
                    on:click={(e) => (encode = !encode)}
                />
                <label
                    for="encode"
                    class="underline select-none"
                    title="Encoding a clip improves playback compatibility and reduces overall filesize, but takes longer to process."
                    >Encode</label
                >
            </div>
        </div>
    </div>
</aside>
<div class="absolute flex flex-col justify-center items-center w-full -translate-y-[3.8rem]">
    <Button
        class="font-extrabold bg-black dark:bg-white dark:text-black"
        buttonClass="font-extrabold"
        on:click={submitForm}>Record</Button
    >
</div>

{#if statusFlash}
    <div class="flex justify-center">
        <Toast
            dismissable={true}
            contentClass="flex space-x-4 rtl:space-x-reverse divide-x rtl:divide-x-reverse divide-gray-200 dark:divide-gray-700"
        >
            <PaperPlaneOutline class="w-5 h-5 rotate-45 text-primary-600 dark:text-primary-500" />
            <div class="text-sm font-normal ps-4">{@html statusFlash.message}</div>
        </Toast>
    </div>
{/if}

{#if !recordings.obtained}
    <p class="italic text-center">Fetching recordings...</p>
{/if}
{#if recordings.error !== undefined}
    <p class="text-center font-bold">
        Failed to fetch recordings: {recordings.error.length > 0
            ? recordings.error
            : "Server is down"}
    </p>
{/if}
{#if recordings.recordings?.length === 0}
    <p class="italic text-center">No recordings yet!</p>
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
                        {recording.channel}
                    </td>
                    <td class="p-2 border-2 border-black dark:border-white">
                        {Stage[recording.stage]}
                        {recording.short_status.length > 0 ? `(${recording.short_status})` : ""}
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
                                Recorded from: <strong>{recording.channel}</strong>
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
                                <p>
                                    <strong>{Stage[recording.stage]}</strong>
                                    {recording.status ? `: ${recording.status}` : ""}
                                </p>
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

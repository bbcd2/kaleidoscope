<script lang="ts">
    import moment from "moment-timezone";
    import { onMount } from "svelte";
    import { enhance } from "$app/forms";
    import { page } from "$app/stores";
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

    // recording uuid (str): show modal (bool)
    let showRecordingModals: { [key: string]: boolean } = {};

    import {
        ArrowRightOutline,
        LinkOutline,
        PaperPlaneOutline,
        BullhornSolid,
    } from "flowbite-svelte-icons";
    import { DurationUnit, SOURCES, Status, lookupSourceById } from "$lib";

    /** Get some recordings */
    const getRecordings = async () => {
        const { data, error } = await supabase
            .from("recordings")
            .select("*")
            .limit(15)
            .order("created_at", { ascending: false });
        if (error) recordings = { obtained: false, error: `${error}`, recordings: undefined };
        else recordings = { obtained: true, error: undefined, recordings: data! };
    };
    /** Get the maximum day for a given month (between 1-12) */
    const getMaxDay = (year: number, month: number): number => {
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
    /** Submit the hidden form */
    const submitForm = async () => {
        const form = document.querySelector("form");
        form?.submit();
    };
    /** Flash form submission status to snackbar if available */
    const flashSnackbar = () => {
        if (!form) return;
        const snackbar = document.querySelector("#snackbar") as HTMLElement;
        if (!snackbar) return;
        switch (form?.status) {
            case 200:
                snackbar.className = "snackbar primary";
                snackbar.innerText = "Successfully submitted form";
                break;
            case 400:
                snackbar.className = "snackbar error";
                snackbar.innerText = form?.body || "Failed to submit form";
                snackbar.classList.add("active");
                break;
        }
        snackbar.classList.add("active");
        setTimeout(() => {
            snackbar.classList.remove("active");
        }, 3000);
    };
    /** Check status on current jobs */
    let statusInterval: NodeJS.Timeout | undefined = undefined;
    const checkCurrentJobStatus = () => {
        if (statusInterval) return;

        statusInterval = setInterval(async () => {
            if (currentJobs.length < 0) return clearInterval(statusInterval);

            for (let jobIdx = currentJobs.length - 1; jobIdx >= 0; jobIdx--) {
                const currentJob = currentJobs[jobIdx];

                // Find the job information
                const { data, error } = await supabase
                    .from("recordings")
                    .select("*")
                    .eq("uuid", currentJob)
                    .limit(1);
                if (error) {
                    console.error(`Error fetching recording status: ${error.message}`);
                    recordings.error = error.message;
                    return clearInterval(statusInterval);
                }

                const newRecordingInfo = data[0]!;

                if (recordings.obtained) {
                    const idx = recordings.recordings!.findIndex(
                        (recording) => recording.uuid == newRecordingInfo.uuid,
                    );
                    if (idx !== -1) recordings.recordings![idx] = newRecordingInfo;
                    else recordings.recordings!.splice(0, 0, newRecordingInfo);
                }
                recordings = recordings;

                // Remove this job if complete
                if (newRecordingInfo.status >= Status.Completed) {
                    currentJobs.splice(jobIdx, 1);
                }
            }
        }, 1000);
    };

    // On mount!
    onMount(() => {
        form && flashSnackbar();
    });

    // Form and database
    interface RecordingsInfo {
        obtained: boolean;
        error: string | undefined;
        recordings: any[] | undefined;
    }
    let recordings: RecordingsInfo = { obtained: false, error: undefined, recordings: undefined };
    let currentJobs: string[] = [];
    $: {
        if (currentJobs.length >= 0) checkCurrentJobStatus();
    }
    export let data;
    $: ({ supabase } = data);
    export let form;

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

    // Current jobs
    // The backend returns the job UUID in its body
    /** @type {import('./$types').Actions} */
    if (form && form.body) {
        currentJobs.push(form.body);
    }
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
    <p class="pb-4">Status:</p>
</div>
<div class="flex justify-center">
    {#if form?.error || form?.status == 200}
        <Toast
            dismissable={true}
            contentClass="flex space-x-4 rtl:space-x-reverse divide-x rtl:divide-x-reverse divide-gray-200 dark:divide-gray-700"
        >
            <PaperPlaneOutline class="w-5 h-5 rotate-45 text-primary-600 dark:text-primary-500" />
            <div class="text-sm font-normal ps-4">
                {#if form?.error}
                    {form?.error.toString()}
                {/if}
                {#if form?.status === 200}
                    <div class="text-sm font-normal ps-4">Your recording has been scheduled.</div>
                {/if}
            </div>
        </Toast>
    {/if}
</div>

<form method="post" use:enhance>
    <input type="hidden" name="startTimestamp" bind:value={startTimestamp} required />
    <input type="hidden" name="endTimestamp" bind:value={endTimestamp} required />
    <input type="hidden" name="channel" bind:value={channel} required />
    <input type="hidden" name="encode" bind:value={encode} required />
</form>

<div class="space" />

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
                            const startDate = new Date(recording.rec_start * 1000);
                            const endDate = new Date(recording.rec_end * 1000);
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
                            const startDate = new Date(recording.rec_start * 1000);
                            const endDate = new Date(recording.rec_end * 1000);
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
                        {Status[recording.status]}
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
                            <p title={recording.user}>Recorded by: <strong>bbcduser</strong></p>
                            <!-- TODO: Have the username change, but thats not entirely needed right now -->
                            <p>
                                Recorded from: <strong>{lookupSourceById(recording.channel)}</strong
                                >
                            </p>
                            <p>
                                Recording date/time: <strong
                                    >{(() => {
                                        const startDate = new Date(recording.rec_start * 1000);
                                        const endDate = new Date(recording.rec_end * 1000);
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
                                        const startDate = new Date(recording.rec_start * 1000);
                                        const endDate = new Date(recording.rec_end * 1000);
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

                            {#if recording.status != Status.Completed}
                                <p>Recording status: <strong>{Status[recording.status]}</strong></p>
                            {/if}

                            <hr />

                            {#if recording.status < Status.Completed}
                                <!-- Show progressbar if not complete -->
                                <Progressbar
                                    progress={Math.min(
                                        100,
                                        recording.status * (100 / (Status["_SENTINEL_MAX_OK"] - 1)),
                                    )}
                                    color="gray"
                                />
                            {:else if recording.status == Status.Completed}
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

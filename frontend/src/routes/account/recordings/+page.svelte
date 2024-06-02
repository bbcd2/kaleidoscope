<script lang="ts">
    $: ({ session, supabase } = data);
    export let data;

    const getRecordings = async () => {
        const { data, error } = await supabase
            .from("recordings")
            .select("* WHERE user_id = $1", session.user.id)
            .limit(15)
            .order("created_at", { ascending: false });
        if (error) console.log("error", error.message);
        return data;
    };
</script>

<h2>Your Recordings</h2>
{#await getRecordings()}
    <p>Fetching recordings...</p>
{:then recordings}
    <table class="border scroll">
        <tr>
            <th>Start Time</th>
            <th>End Time</th>
            <th>Channel</th>
            <th>Status</th>
            <th>Download</th>
        </tr>

        {#each recordings ?? [] as recording}
            <tr>
                <td
                    >{new Date(recording.rec_start * 1000).toLocaleString("en-GB", {
                        timeZone: "Europe/London",
                    })}</td
                >
                <td
                    >{new Date(recording.rec_end * 1000).toLocaleString("en-GB", {
                        timeZone: "Europe/London",
                    })}</td
                >
                <td>{channelString(parseInt(recording.channel))}</td>
                <td>{statusString(recording.status)}</td>
                <td><u><a href="/express/download/{recording.uuid}">Click</a></u></td>
            </tr>
        {/each}
    </table>
{:catch error}
    <p>Failed to fetch recordings: {error.message}</p>
{/await}

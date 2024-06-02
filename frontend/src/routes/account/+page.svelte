<script lang="ts">
    import { page } from "$app/stores";
    import { get } from "svelte/store";

    const code = $page.url.searchParams.get("code");
    let username: string;
    if (code) {
        fetch("https://discord.com/api/oauth2/token", {
            method: "POST",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
                // Authenticated with the client ID and secret
                Authorization:
                    "Basic " + btoa("1151344165892935700:t62FNY-EJFb_owiMP-TBZMlyC5KH9zlS"),
            },
            credentials: "include",
            body:
                "grant_type=authorization_code&code=" +
                code +
                "&redirect_uri=http%3A%2F%2Flocalhost%3A5173%2Faccount&scope=identify",
            /* new URLSearchParams({
                grant_type: "authorization_code",
                code,
                redirect_uri: 'http://localhost:5173/account',
                scope: 'identify',
            }), */
        })
            .then((result) => result.json())
            .then((response) => {
                const { access_token, token_type } = response;
                fetch("https://discord.com/api/users/@me", {
                    headers: {
                        authorization: `${token_type} ${access_token}`,
                    },
                })
                    .then((result) => result.json())
                    .then((response) => {
                        username = response.username;
                    })
                    .catch(console.error);
            })
            .catch(console.error);
    }
</script>

<div id="info">Hellow!</div>
{#if username}
    <div id="info">Hellow, {username}!</div>
{:else}
    <a
        id="info"
        href="https://discord.com/oauth2/authorize?client_id=1151344165892935700&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A5173%2Faccount&scope=identify&prompt=none"
        >Please log in with Discord to view this page.</a
    >
{/if}

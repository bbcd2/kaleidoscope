<script lang="ts">
    import { page } from "$app/stores";

    import "../app.css";
    import { Navbar, NavBrand, NavLi, NavUl, NavHamburger, DarkMode } from "flowbite-svelte";

    $: activeUrl = $page.url.pathname;

    const btnClass =
        "text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg text-xl p-2 center-content";
</script>

<div class="flex flex-col min-h-screen">
    <Navbar class="border-b border-b-black dark:border-b-white dark:bg-black">
        <NavBrand href="/">
            <img src="logo.svg" class="h-6 me-3 sm:h-9 dark:invert" alt="BBCD Logo" />
        </NavBrand>
        <NavHamburger />
        <NavUl {activeUrl}>
            <NavLi>
                <DarkMode {btnClass} />
            </NavLi>
        </NavUl>
    </Navbar>
    <main class="flex-1">
        <slot />
    </main>
    <div class="w-full mt-5 border-t border-black dark:border-white">
        <p class="m-5 text-center">Made with ❤️ by Kat 🏳️‍⚧️</p>
    </div>
</div>

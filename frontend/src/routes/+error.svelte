<script>
    import { page } from "$app/stores";
</script>

<h1 class="mt-5 text-4xl text-center">{$page.status}</h1>
<h2 class="text-center">{$page.error?.message}</h2>

<script>
  import { add } from "toy-hashgraph-js";

  let a = $state(5);
  let b = $state(7);
  let result = $derived(add(a, b));
</script>

<main class="text-center p-8 max-w-2xl mx-auto">
  <h1 class="text-4xl font-bold mb-8">Toy Hashgraph WASM Example</h1>

  <div class="flex flex-col gap-4 mt-8">
    <label class="flex flex-col gap-2">
      <span>Number A:</span>
      <input
        type="number"
        bind:value={a}
        class="px-3 py-2 text-base border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </label>

    <label class="flex flex-col gap-2">
      <span>Number B:</span>
      <input
        type="number"
        bind:value={b}
        class="px-3 py-2 text-base border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </label>

    <div class="mt-4 p-4 bg-gray-100 rounded-md text-2xl">
      <strong>{a} + {b} = {result}</strong>
    </div>
  </div>
</main>

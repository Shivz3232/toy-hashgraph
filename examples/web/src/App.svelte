<script>
  import { Hashgraph } from "toy-hashgraph";

  $effect(() => {
    (async () => {
      const peers = ["a", "b", "c", "d"];
      const publicKeys = new Map();
      const privateKeys = new Map();
      for (const peer of peers) {
        const key = await crypto.subtle.generateKey("Ed25519", true, [
          "sign",
          "verify",
        ]);
        const publicKey = new Uint8Array(
          await crypto.subtle.exportKey("raw", key.publicKey)
        );
        const privateKey = new Uint8Array(
          await crypto.subtle.exportKey("pkcs8", key.privateKey)
        );

        publicKeys.set(peer, publicKey);
        privateKeys.set(peer, privateKey);
      }

      const hashgraph = new Hashgraph("a", privateKeys.get("a"), publicKeys);
    })();
  });
</script>

<main class="text-center p-8 max-w-2xl mx-auto">
  <h1 class="text-4xl font-bold mb-8">Toy Hashgraph WASM Example</h1>

  <div class="flex flex-col gap-4 mt-8"></div>
</main>

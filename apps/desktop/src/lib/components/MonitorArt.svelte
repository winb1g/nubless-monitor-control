<script lang="ts">
  // A neutral monitor drawing (no vendor artwork) for the empty state: all
  // dashed, "no signal", with a cursor block blinking. Decorative.
</script>

<div class="art" aria-hidden="true">
  <div class="bezel">
    <div class="screen"><span class="msg">No signal</span><span class="cursor"></span></div>
  </div>
  <div class="neck"></div>
  <div class="foot"></div>
</div>

<style>
  .art {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
  }
  .bezel {
    width: 100%;
    padding: 14px;
    border: 2px dashed var(--ink);
    border-radius: 10px;
  }
  .screen {
    aspect-ratio: 16 / 9;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    border: 2px dashed var(--rule);
    border-radius: 3px;
  }
  .msg {
    font-family: var(--font-mono);
    font-size: var(--fs-md);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--dim);
  }
  .cursor {
    width: 10px;
    height: 20px;
    background: var(--ink);
    animation: blink var(--blink) steps(2, jump-none) infinite;
  }
  .neck {
    width: 72px;
    height: 28px;
    border-left: 2px dashed var(--ink);
    border-right: 2px dashed var(--ink);
  }
  .foot {
    width: 220px;
    max-width: 60%;
    height: 10px;
    border: 2px dashed var(--ink);
    border-radius: 5px;
  }
</style>

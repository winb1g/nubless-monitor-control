<script lang="ts">
  // A neutral monitor illustration (no vendor artwork). Swap for a product image later.
  let { size = 120, glow = true, dim = false }: { size?: number; glow?: boolean; dim?: boolean } = $props();
</script>

<svg width={size} height={size * 0.78} viewBox="0 0 160 125" class:dim aria-hidden="true">
  <defs>
    <linearGradient id="scr" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#7b61ff" />
      <stop offset="0.55" stop-color="#4f8dff" />
      <stop offset="1" stop-color="#22c3ee" />
    </linearGradient>
    <linearGradient id="shine" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#fff" stop-opacity="0" />
      <stop offset="0.5" stop-color="#fff" stop-opacity="0.35" />
      <stop offset="1" stop-color="#fff" stop-opacity="0" />
    </linearGradient>
    <clipPath id="screen"><rect x="9" y="9" width="142" height="80" rx="4.5" /></clipPath>
  </defs>
  {#if glow}<ellipse cx="80" cy="116" rx="52" ry="5" fill="#7b61ff" opacity="0.18" />{/if}
  <rect x="6" y="6" width="148" height="86" rx="7" fill="#1b1e29" />
  <rect x="9" y="9" width="142" height="80" rx="4.5" fill="url(#scr)" />
  <g clip-path="url(#screen)"><rect x="-40" y="9" width="40" height="80" fill="url(#shine)" class="sweep" /></g>
  <path d="M70 92 h20 l5 18 h-30 z" fill="#2a2e3c" />
  <rect x="52" y="108" width="56" height="6" rx="3" fill="#2a2e3c" />
  <circle cx="80" cy="95.5" r="1.6" fill="#7b61ff" class="nub" />
</svg>

<style>
  svg {
    display: block;
    overflow: visible;
  }
  .dim {
    filter: grayscale(0.9) opacity(0.55);
  }
  .sweep {
    animation: sweep 5s var(--ease) infinite;
  }
  @keyframes sweep {
    0%,
    70% {
      transform: translateX(0);
    }
    100% {
      transform: translateX(200px);
    }
  }
  .nub {
    animation: pulse-nub 2.4s ease-in-out infinite;
  }
  @keyframes pulse-nub {
    50% {
      opacity: 0.3;
    }
  }
</style>

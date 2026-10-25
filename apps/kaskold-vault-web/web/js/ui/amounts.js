// Exact sompi/KAS display formatting for review screens (BigInt, no floats).

const SOMPI_PER_KAS = 100_000_000n;

function groupInteger(value) {
  return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

export function formatSompi(value) {
  return groupInteger(BigInt(String(value || '0')).toString());
}

export function formatKas(value) {
  const sompi = BigInt(String(value || '0'));
  const whole = sompi / SOMPI_PER_KAS;
  const fraction = (sompi % SOMPI_PER_KAS).toString().padStart(8, '0');
  return `${groupInteger(whole.toString())}.${fraction}`;
}

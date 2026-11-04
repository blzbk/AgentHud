/** Display formatting of usage values. Times are seconds since the epoch. */

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const timeFormat = new Intl.DateTimeFormat("fr-FR", { hour: "2-digit", minute: "2-digit" });
const dayFormat = new Intl.DateTimeFormat("fr-FR", { weekday: "short" });

/** Rounded percentage, such as `41 %`. */
export function formatPercent(percent: number): string {
  return `${Math.round(percent)} %`;
}

/** Reset time: the hour within the next 24 h, the weekday beyond. */
export function formatReset(resetsAt: number, now: number): string {
  const date = new Date(resetsAt * 1000);
  return resetsAt - now < DAY ? timeFormat.format(date) : dayFormat.format(date);
}

/** Age of the data, such as `il y a 3 min`. */
export function formatAge(observedAt: number, now: number): string {
  const age = now - observedAt;
  if (age < MINUTE) return "à l'instant";
  if (age < HOUR) return `il y a ${Math.floor(age / MINUTE)} min`;
  if (age < DAY) return `il y a ${Math.floor(age / HOUR)} h`;
  return `il y a ${Math.floor(age / DAY)} j`;
}

/** Current time in seconds since the epoch. */
export function nowInSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

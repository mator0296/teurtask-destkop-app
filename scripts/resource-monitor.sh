#!/usr/bin/env bash
# Samples CPU and memory of the teurtask desktop app (main process + WebKit children) on Linux.
#
#   ./resource-monitor.sh [seconds=60] [interval=2] [out.csv]
#
# Prints one line per sample and a summary (avg/max CPU, avg/max RSS) per process at the end.
# CPU is per-interval (delta of utime+stime), not the lifetime average `ps` reports.
set -euo pipefail

DURATION=${1:-60}
INTERVAL=${2:-2}
OUT=${3:-}
TICKS=$(getconf CLK_TCK)

main_pid=$(pgrep -x teurtask | head -1 || true)
if [[ -z "$main_pid" ]]; then
  echo "teurtask is not running" >&2
  exit 1
fi

pids() {
  echo "$main_pid"
  pgrep -P "$main_pid" || true
}

cpu_ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
rss_kb() { awk '/^VmRSS/ {print $2}' "/proc/$1/status" 2>/dev/null || echo 0; }
name_of() {
  case "$(cat "/proc/$1/comm" 2>/dev/null)" in
    WebKitWebProces*) echo web ;;
    WebKitNetworkPr*) echo network ;;
    WebKitGPUProces*) echo gpu ;;
    teurtask) echo main ;;
    *) cat "/proc/$1/comm" ;;
  esac
}

[[ -n "$OUT" ]] && echo "ts,process,pid,cpu_pct,rss_mb" > "$OUT"

declare -A prev cpu_sum cpu_max rss_sum rss_max samples
for p in $(pids); do prev[$p]=$(cpu_ticks "$p"); done

end=$(( $(date +%s) + DURATION ))
while (( $(date +%s) < end )); do
  sleep "$INTERVAL"
  ts=$(date +%H:%M:%S)
  line="$ts"
  total_cpu=0
  total_rss=0
  for p in $(pids); do
    now=$(cpu_ticks "$p")
    cpu=$(awk -v a="${prev[$p]:-$now}" -v b="$now" -v t="$TICKS" -v i="$INTERVAL" \
      'BEGIN {printf "%.1f", (b - a) / t / i * 100}')
    prev[$p]=$now
    rss=$(( $(rss_kb "$p") / 1024 ))
    n=$(name_of "$p")
    line+="  $n=${cpu}%/${rss}MB"
    total_cpu=$(awk -v a="$total_cpu" -v b="$cpu" 'BEGIN {print a + b}')
    total_rss=$(( total_rss + rss ))
    cpu_sum[$n]=$(awk -v a="${cpu_sum[$n]:-0}" -v b="$cpu" 'BEGIN {print a + b}')
    cpu_max[$n]=$(awk -v a="${cpu_max[$n]:-0}" -v b="$cpu" 'BEGIN {print (b > a ? b : a)}')
    rss_sum[$n]=$(( ${rss_sum[$n]:-0} + rss ))
    (( rss > ${rss_max[$n]:-0} )) && rss_max[$n]=$rss
    samples[$n]=$(( ${samples[$n]:-0} + 1 ))
    [[ -n "$OUT" ]] && echo "$ts,$n,$p,$cpu,$rss" >> "$OUT"
  done
  echo "$line  | total=${total_cpu}%/${total_rss}MB"
done

echo
printf '%-8s %8s %8s %10s %10s\n' process cpu_avg cpu_max rss_avg_mb rss_max_mb
for n in "${!samples[@]}"; do
  printf '%-8s %7.1f%% %7.1f%% %10d %10d\n' "$n" \
    "$(awk -v s="${cpu_sum[$n]}" -v c="${samples[$n]}" 'BEGIN {print s / c}')" \
    "${cpu_max[$n]}" $(( rss_sum[$n] / samples[$n] )) "${rss_max[$n]}"
done

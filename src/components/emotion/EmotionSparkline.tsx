interface EmotionSparklineProps {
  data: number[];
  color: string;
  width?: number;
  height?: number;
}

export function EmotionSparkline({
  data,
  color,
  width = 40,
  height = 16,
}: EmotionSparklineProps) {
  if (data.length === 0) return null;

  if (data.length === 1) {
    const y = height - (data[0] / 100) * height;
    return (
      <svg width={width} height={height} className="shrink-0">
        <circle cx={width / 2} cy={y} r={2} fill={color} />
      </svg>
    );
  }

  const step = width / (data.length - 1);
  const points = data
    .map((v, i) => {
      const x = i * step;
      const y = height - (v / 100) * height;
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <svg width={width} height={height} className="shrink-0">
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

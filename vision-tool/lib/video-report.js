// vision-tool/lib/video-report.js
//
// Builds structured timeline segments and human-readable reports
// from video analysis batch results. No vscode dependencies.

function alignTranscriptToTimestamp(timestamp, transcript, windowSec = 2) {
  if (transcript.type === "none") {
    return { text: null, confidence: 0 };
  }

  if (transcript.type === "raw") {
    return { text: null, confidence: 0 };
  }

  const matching = transcript.segments.filter((seg) => {
    const segStart = seg.start || seg.startSec || seg.start_sec || 0;
    const segEnd = seg.end || seg.endSec || seg.end_sec || segStart + windowSec;
    return timestamp >= segStart - windowSec && timestamp <= segEnd + windowSec;
  });

  if (matching.length > 0) {
    const text = matching
      .map((s) => s.text || s.transcript || "")
      .filter(Boolean)
      .join(" ");
    return { text: text || null, confidence: 0.9 };
  }

  return { text: null, confidence: 0 };
}

function extractFrameAnalysis(fullAnalysis, frame) {
  if (!fullAnalysis) return null;

  const tsFixed = frame.timestamp.toFixed(2);
  const tsShort = frame.timestamp.toFixed(1);

  const patterns = [
    new RegExp(
      `${tsFixed.replace(".", "\\.")}s[:\\s]([\\s\\S]*?)(?=\\d+\\.\\d+s[:\\s]|$)`,
      "i",
    ),
    new RegExp(
      `${frame.filename.replace(".", "\\.")}[:\\s]([\\s\\S]*?)(?=frame_|$)`,
      "i",
    ),
    new RegExp(
      `Frame.*?${tsShort.replace(".", "\\.")}[:\\s]([\\s\\S]*?)(?=Frame|$)`,
      "i",
    ),
  ];

  for (const pattern of patterns) {
    const match = fullAnalysis.match(pattern);
    if (match && match[1] && match[1].trim().length > 20) {
      return match[1].trim();
    }
  }

  return null;
}

function extractOcrText(analysis) {
  if (!analysis) return null;

  const patterns = [
    /text\s*(?:reads?|says?|shows?|displays?)[:\s]*["']?([^"'\n]+)/gi,
    /on-screen\s+text[:\s]*["']?([^"'\n]+)/gi,
    /visible\s+text[:\s]*["']?([^"'\n]+)/gi,
    /caption[:\s]*["']?([^"'\n]+)/gi,
  ];

  const extracted = [];
  for (const pattern of patterns) {
    let match;
    while ((match = pattern.exec(analysis)) !== null) {
      const text = match[1].trim();
      if (text.length > 2 && !extracted.includes(text)) {
        extracted.push(text);
      }
    }
  }

  return extracted.length > 0 ? extracted.join("; ") : null;
}

function buildTimeline(batchResults, transcript, metadata) {
  const segments = [];

  for (const batch of batchResults) {
    for (const frame of batch.frames) {
      const ts = frame.timestamp;
      const aligned = alignTranscriptToTimestamp(ts, transcript);
      const frameAnalysis = extractFrameAnalysis(batch.analysis, frame);

      segments.push({
        start: ts,
        end: Math.min(ts + 1, metadata.durationSec),
        transcript:
          aligned.text ||
          (transcript.type === "raw" ? "[see raw transcript]" : null),
        visual: frameAnalysis || batch.analysis.slice(0, 300),
        ocrLikeText: extractOcrText(frameAnalysis || batch.analysis),
        confidence: aligned.text
          ? aligned.confidence
          : transcript.type === "none"
            ? 1
            : 0.3,
      });
    }
  }

  return segments;
}

function buildReport(
  videoPath,
  metadata,
  plan,
  segments,
  globalSummary,
  transcript,
) {
  const lines = [];

  lines.push("# Video Analysis Report");
  lines.push("");
  lines.push("## Source");
  lines.push(`- **Path**: ${videoPath}`);
  lines.push(`- **Duration**: ${metadata.durationSec.toFixed(1)}s`);
  lines.push(`- **Resolution**: ${metadata.width}x${metadata.height}`);
  lines.push(`- **FPS**: ${metadata.fps.toFixed(1)}`);
  lines.push(`- **Codec**: ${metadata.codec}`);
  lines.push("");

  lines.push("## Sampling");
  lines.push(`- **Strategy**: ${plan.strategy}`);
  lines.push(`- **Interval**: ${plan.interval}s`);
  lines.push(`- **Frames analyzed**: ${plan.frameCount}`);
  lines.push(
    `- **Window**: ${plan.startSec.toFixed(1)}s – ${plan.endSec.toFixed(1)}s`,
  );
  lines.push("");

  if (transcript.type === "raw") {
    lines.push("## Transcript (raw, unaligned)");
    lines.push(transcript.text);
    lines.push("");
  } else if (transcript.type === "segmented") {
    lines.push(
      `## Transcript (${transcript.segments.length} segments, aligned)`,
    );
    lines.push("");
  } else {
    lines.push("## Transcript");
    lines.push("_No transcript provided._");
    lines.push("");
  }

  lines.push("## Visual Timeline");
  lines.push("");

  for (const seg of segments) {
    lines.push(`### ${seg.start.toFixed(1)}s – ${seg.end.toFixed(1)}s`);

    if (seg.visual) {
      lines.push(seg.visual);
    }
    if (seg.transcript && seg.transcript !== "[see raw transcript]") {
      lines.push(`> **Speech**: ${seg.transcript}`);
    }
    if (seg.ocrLikeText) {
      lines.push(`> **On-screen text**: ${seg.ocrLikeText}`);
    }
    lines.push("");
  }

  lines.push("## Summary");
  lines.push(globalSummary);
  lines.push("");

  return lines.join("\n");
}

module.exports = {
  buildTimeline,
  buildReport,
  alignTranscriptToTimestamp,
  extractOcrText,
};

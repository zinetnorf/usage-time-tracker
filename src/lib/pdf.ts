import { jsPDF } from "jspdf";
import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { api } from "./api";
import { formatDuration, type AppDayUsage, type DayTotals } from "./format";

/** Filas sin apps blacklisteadas: los reportes no las incluyen. */
export async function withoutBlacklisted(rows: AppDayUsage[]): Promise<AppDayUsage[]> {
  const apps = await api.apps();
  const excluded = new Set(apps.filter((a) => a.blacklisted).map((a) => a.id));
  return rows.filter((r) => !excluded.has(r.app_id));
}

/** Rasteriza el SVG de un chart Recharts a PNG (escala 2x). */
export function svgToPng(svg: SVGSVGElement): Promise<{ dataUrl: string; ratio: number }> {
  const rect = svg.getBoundingClientRect();
  const clone = svg.cloneNode(true) as SVGSVGElement;
  clone.setAttribute("width", String(rect.width));
  clone.setAttribute("height", String(rect.height));
  const xml = new XMLSerializer().serializeToString(clone);
  const url = URL.createObjectURL(new Blob([xml], { type: "image/svg+xml" }));

  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = rect.width * 2;
      canvas.height = rect.height * 2;
      const ctx = canvas.getContext("2d");
      if (!ctx) return reject(new Error("canvas 2d no disponible"));
      // Fondo oscuro del dashboard para que el chart no quede transparente.
      ctx.fillStyle = "#09090b";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
      URL.revokeObjectURL(url);
      resolve({ dataUrl: canvas.toDataURL("image/png"), ratio: rect.height / rect.width });
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("no se pudo rasterizar el chart"));
    };
    img.src = url;
  });
}

interface Labels {
  app: string;
  active: string;
  idle: string;
  total: string;
  day: string;
}

const MARGIN = 14;
const PAGE_W = 210; // A4 vertical, mm
const PAGE_H = 297;

class Report {
  doc = new jsPDF();
  y = MARGIN;

  private ensureRoom(height: number) {
    if (this.y + height > PAGE_H - MARGIN) {
      this.doc.addPage();
      this.y = MARGIN;
    }
  }

  title(text: string) {
    this.doc.setFontSize(16).setFont("helvetica", "bold");
    this.doc.text(text, MARGIN, this.y + 6);
    this.y += 10;
  }

  subtitle(text: string) {
    this.doc.setFontSize(10).setFont("helvetica", "normal").setTextColor(110);
    this.doc.text(text, MARGIN, this.y + 4);
    this.doc.setTextColor(0);
    this.y += 8;
  }

  section(text: string) {
    this.ensureRoom(12);
    this.doc.setFontSize(12).setFont("helvetica", "bold");
    this.doc.text(text, MARGIN, this.y + 6);
    this.y += 9;
  }

  chart(png: { dataUrl: string; ratio: number }) {
    const width = PAGE_W - MARGIN * 2;
    const height = width * png.ratio;
    this.ensureRoom(height + 4);
    this.doc.addImage(png.dataUrl, "PNG", MARGIN, this.y, width, height);
    this.y += height + 4;
  }

  appTable(rows: AppDayUsage[], labels: Labels) {
    const col = [MARGIN, PAGE_W - 90, PAGE_W - 55, PAGE_W - MARGIN];
    this.tableHeader([labels.app, labels.active, labels.idle], col);
    this.doc.setFont("helvetica", "normal").setFontSize(9);
    for (const r of rows) {
      this.ensureRoom(6);
      const name = this.doc.splitTextToSize(r.display_name, col[1] - col[0] - 4)[0];
      this.doc.text(String(name), col[0], this.y + 4);
      this.doc.text(formatDuration(r.active_sec), col[2] - 2, this.y + 4, { align: "right" });
      this.doc.text(formatDuration(r.idle_sec), col[3] - 2, this.y + 4, { align: "right" });
      this.y += 6;
    }
    this.y += 2;
  }

  dayTable(rows: DayTotals[], labels: Labels) {
    const col = [MARGIN, PAGE_W - 90, PAGE_W - 55, PAGE_W - MARGIN];
    this.tableHeader([labels.day, labels.active, labels.idle], col);
    this.doc.setFont("helvetica", "normal").setFontSize(9);
    for (const r of rows) {
      this.ensureRoom(6);
      this.doc.text(r.day, col[0], this.y + 4);
      this.doc.text(formatDuration(r.active_sec), col[2] - 2, this.y + 4, { align: "right" });
      this.doc.text(formatDuration(r.idle_sec), col[3] - 2, this.y + 4, { align: "right" });
      this.y += 6;
    }
    this.y += 2;
  }

  private tableHeader(titles: string[], col: number[]) {
    this.ensureRoom(8);
    this.doc.setFont("helvetica", "bold").setFontSize(9);
    this.doc.text(titles[0], col[0], this.y + 4);
    this.doc.text(titles[1], col[2] - 2, this.y + 4, { align: "right" });
    this.doc.text(titles[2], col[3] - 2, this.y + 4, { align: "right" });
    this.doc.line(MARGIN, this.y + 5.5, PAGE_W - MARGIN, this.y + 5.5);
    this.y += 7;
  }

  totalLine(label: string, totalSec: number) {
    this.ensureRoom(8);
    this.doc.setFont("helvetica", "bold").setFontSize(11);
    this.doc.text(`${label}: ${formatDuration(totalSec)}`, MARGIN, this.y + 5);
    this.y += 8;
  }
}

export interface ReportSection {
  section: string;
  chart?: SVGSVGElement | null;
  apps?: AppDayUsage[];
  days?: DayTotals[];
  total?: { label: string; sec: number };
}

/** Compone el PDF y abre "Guardar como". true si se guardó. */
export async function exportReport(
  title: string,
  subtitle: string,
  sections: ReportSection[],
  labels: Labels,
  defaultName: string,
): Promise<boolean> {
  const report = new Report();
  report.title(title);
  report.subtitle(subtitle);

  for (const s of sections) {
    report.section(s.section);
    if (s.total) report.totalLine(s.total.label, s.total.sec);
    if (s.chart) report.chart(await svgToPng(s.chart));
    if (s.apps?.length) report.appTable(s.apps, labels);
    if (s.days?.length) report.dayTable(s.days, labels);
  }

  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!path) return false;

  const bytes = Array.from(new Uint8Array(report.doc.output("arraybuffer")));
  await invoke("save_file", { path, bytes });
  return true;
}

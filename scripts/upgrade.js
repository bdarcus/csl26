const fs = require("fs");
const yaml = require("js-yaml");
const path = require("path");

const report = require("/tmp/core-report.json");
const needsUpgrade = report.styles.filter(s => s.fidelityScore < 0.95 || s.qualityScore < 0.90);

console.log(`Upgrading ${needsUpgrade.length} styles...`);

for (const s of needsUpgrade) {
    const yamlPath = path.join(__dirname, "..", "styles", `${s.name}.yaml`);
    if (!fs.existsSync(yamlPath)) continue;
    let content = fs.readFileSync(yamlPath, "utf8");
    let baseData;
    try {
        baseData = yaml.load(content);
    } catch(e) {
        continue;
    }
    let options = baseData.options;
    if (!options) continue;
    let modified = false;

    // 1. Substitute preset: standard
    if (options.substitute && typeof options.substitute === "object") {
        const t = options.substitute.candidates;
        if (t && t.length === 3 && t[0] === "editor" && t[1] === "title" && t[2] === "translator" && !options.substitute["contributor-role-form"]) {
            options.substitute = "standard";
            modified = true;
        }
    }

    // 2. Dates preset
    if (options.dates && typeof options.dates === "object") {
        if (!options.dates["uncertainty-marker"] && !options.dates["approximation-marker"]) {
            if (options.dates.month === "long") { options.dates = "long"; modified = true; }
            else if (options.dates.month === "numeric") { options.dates = "numeric"; modified = true; }
            else if (options.dates.month === "short") { options.dates = "short"; modified = true; }
        }
    }

    // 3. Titles preset
    if (options.titles && typeof options.titles === "object") {
        const t = options.titles;
        if (t.periodical && t.periodical.emph && t.serial && t.serial.emph) {
            if (t.monograph && t.monograph.emph && (!t.component || !t.component.quote)) {
                options.titles = "humanities";
                modified = true;
            } else if ((!t.monograph || !t.monograph.emph) && (!t.component || !t.component.quote)) {
                options.titles = "journal-emphasis";
                modified = true;
            }
        } else if (
            (!t.component || !t.component.emph) && (!t.component || !t.component.quote) &&
            (!t.monograph || !t.monograph.emph) && (!t.monograph || !t.monograph.quote) &&
            (!t.periodical || !t.periodical.emph) && (!t.periodical || !t.periodical.quote)
        ) {
            options.titles = "scientific";
            modified = true;
        }
    }

    // Apply safe text replacements rather than re-emitting the full YAML structure.
    if (modified) {
        if (options.substitute === "standard") {
            content = content.replace(/substitute:\n\s+candidates:\n\s+- editor\n\s+- title\n\s+- translator/g, 'substitute: standard');
        }
        if (options.dates === "long") {
            content = content.replace(/dates:\n\s+month: long/g, 'dates: long');
        } else if (options.dates === "numeric") {
            content = content.replace(/dates:\n\s+month: numeric/g, 'dates: numeric');
        } else if (options.dates === "short") {
            content = content.replace(/dates:\n\s+month: short/g, 'dates: short');
        }
        if (options.titles === "humanities") {
            content = content.replace(/titles:\n\s+monograph:\n\s+emph: true\n\s+periodical:\n\s+emph: true\n\s+serial:\n\s+emph: true/g, 'titles: humanities');
        } else if (options.titles === "journal-emphasis") {
            content = content.replace(/titles:\n\s+periodical:\n\s+emph: true\n\s+serial:\n\s+emph: true/g, 'titles: journal-emphasis');
        } else if (options.titles === "scientific") {
            content = content.replace(/titles: \{\}/g, 'titles: scientific');
        }
        
        fs.writeFileSync(yamlPath, content);
        console.log(`Updated presets for ${s.name}`);
    }
}
console.log("Finished preset replacements.");

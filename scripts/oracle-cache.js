#!/usr/bin/env node
/**
 * scripts/oracle-cache.js
 *
 * Caches oracle outputs to save time and tokens.
 *
 * Usage:
 *   node scripts/oracle-cache.js <style-path>
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { execSync } = require('child_process');

const stylePath = process.argv[2];
const CACHE_DIR = path.join(__dirname, '..', '.oracle-cache');

if (!stylePath) {
    console.error('Usage: node scripts/oracle-cache.js <style-path>');
    process.exit(1);
}

if (!fs.existsSync(stylePath)) {
    console.error(`Style not found: ${stylePath}`);
    process.exit(1);
}

if (!fs.existsSync(CACHE_DIR)) {
    fs.mkdirSync(CACHE_DIR);
}

try {
    const styleContent = fs.readFileSync(stylePath, 'utf8');
    const hash = crypto.createHash('md5').update(styleContent).digest('hex');
    const styleName = path.basename(stylePath, '.csl');
    const cachePath = path.join(CACHE_DIR, `${styleName}-${hash}.json`);

    if (fs.existsSync(cachePath)) {
        console.log(`✅ Cache HIT for ${styleName}`);
        console.log(fs.readFileSync(cachePath, 'utf8'));
        process.exit(0);
    }

    console.error(`🌀 Cache MISS for ${styleName}. Running oracle...`);
    const output = execSync(`node scripts/oracle-migration.js "${stylePath}" --json`, { encoding: 'utf8' });

    fs.writeFileSync(cachePath, output);
    console.log(output);

} catch (err) {
    console.error(`❌ Caching failed: ${err.message}`);
    process.exit(1);
}

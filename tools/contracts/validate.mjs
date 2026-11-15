#!/usr/bin/env node
// ตรวจ JSON contract ด้วย validator ขนาดเล็ก (ไม่พึ่ง dependency)
// รองรับ subset ที่ schema ของเราใช้: type/required/enum/const/ref/items/pattern/...
// รัน: node tools/contracts/validate.mjs

import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.resolve(process.cwd());
const DIR = path.join(ROOT, 'project menager', 'Design', '03-engineering', 'contracts');

const schemaOf = {
  hud: read('hud-state.schema.json'),
  config: read('config.schema.json'),
};

const cases = [
  ...read('hud-state.samples.json').map((value, i) => ({
    name: `hud-state.samples[${i}] (${value.mode}/${value.state})`,
    schema: schemaOf.hud,
    value,
  })),
  { name: 'config.sample.json', schema: schemaOf.config, value: read('config.sample.json') },
];

let failed = 0;

for (const c of cases) {
  const errors = validate(c.schema, c.value, c.schema, '$');
  if (errors.length === 0) {
    console.log(`PASS  ${c.name}`);
  } else {
    failed++;
    console.log(`FAIL  ${c.name}`);
    for (const e of errors) console.log(`      ${e}`);
  }
}

console.log(`\n${cases.length - failed}/${cases.length} ผ่าน`);
process.exit(failed === 0 ? 0 : 1);

/* ---------------- tiny JSON Schema subset ---------------- */

function read(file) {
  return JSON.parse(fs.readFileSync(path.join(DIR, file), 'utf8'));
}

function resolveRef(ref, root) {
  if (!ref.startsWith('#/')) throw new Error(`unsupported $ref: ${ref}`);
  return ref
    .slice(2)
    .split('/')
    .reduce((node, key) => node?.[key.replaceAll('~1', '/').replaceAll('~0', '~')], root);
}

function validate(schema, value, root, at) {
  if (schema === true || schema === undefined) return [];
  if (schema === false) return [`${at}: ไม่อนุญาตให้มีค่านี้`];
  if (schema.$ref) return validate(resolveRef(schema.$ref, root), value, root, at);

  const errors = [];
  const type = schema.type;

  if (type && !hasType(value, type)) {
    return [`${at}: ต้องเป็น ${type} (ได้ ${describe(value)})`];
  }
  if ('const' in schema && value !== schema.const) {
    errors.push(`${at}: ต้องเท่ากับ ${JSON.stringify(schema.const)}`);
  }
  if (schema.enum && !schema.enum.includes(value)) {
    errors.push(`${at}: ต้องเป็นหนึ่งใน ${schema.enum.join(' | ')}`);
  }
  if (typeof value === 'number') {
    if (schema.minimum !== undefined && value < schema.minimum) errors.push(`${at}: ต้อง ≥ ${schema.minimum}`);
    if (schema.maximum !== undefined && value > schema.maximum) errors.push(`${at}: ต้อง ≤ ${schema.maximum}`);
  }
  if (typeof value === 'string') {
    if (schema.minLength !== undefined && value.length < schema.minLength) errors.push(`${at}: ยาวอย่างน้อย ${schema.minLength}`);
    if (schema.maxLength !== undefined && value.length > schema.maxLength) errors.push(`${at}: ยาวไม่เกิน ${schema.maxLength}`);
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) errors.push(`${at}: ไม่ตรง pattern ${schema.pattern}`);
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) errors.push(`${at}: ต้องมีอย่างน้อย ${schema.minItems} รายการ`);
    if (schema.maxItems !== undefined && value.length > schema.maxItems) errors.push(`${at}: ต้องมีไม่เกิน ${schema.maxItems} รายการ`);
    if (schema.uniqueItems) {
      const seen = new Set(value.map((v) => JSON.stringify(v)));
      if (seen.size !== value.length) errors.push(`${at}: ต้องไม่ซ้ำกัน`);
    }
    if (schema.items) {
      value.forEach((item, i) => errors.push(...validate(schema.items, item, root, `${at}[${i}]`)));
    }
  }
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    for (const key of schema.required ?? []) {
      if (!(key in value)) errors.push(`${at}.${key}: ต้องมี`);
    }
    const props = schema.properties ?? {};
    for (const [key, child] of Object.entries(value)) {
      if (props[key]) {
        errors.push(...validate(props[key], child, root, `${at}.${key}`));
        continue;
      }
      const patterns = Object.entries(schema.patternProperties ?? {});
      const matched = patterns.filter(([re]) => new RegExp(re).test(key));
      if (matched.length > 0) {
        for (const [, sub] of matched) errors.push(...validate(sub, child, root, `${at}.${key}`));
        continue;
      }
      if (schema.additionalProperties === false) {
        errors.push(`${at}.${key}: field ไม่รู้จัก`);
      } else if (typeof schema.additionalProperties === 'object') {
        errors.push(...validate(schema.additionalProperties, child, root, `${at}.${key}`));
      }
    }
  }
  return errors;
}

function hasType(value, type) {
  switch (type) {
    case 'object': return value !== null && typeof value === 'object' && !Array.isArray(value);
    case 'array': return Array.isArray(value);
    case 'integer': return Number.isInteger(value);
    case 'number': return typeof value === 'number' && Number.isFinite(value);
    case 'string': return typeof value === 'string';
    case 'boolean': return typeof value === 'boolean';
    case 'null': return value === null;
    default: throw new Error(`unsupported type: ${type}`);
  }
}

function describe(value) {
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  return typeof value;
}

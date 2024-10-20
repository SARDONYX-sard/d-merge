// SPDX-License-Identifier: MIT or Apache-2.0
// Copyright (c) 2022 Takefumi Yoshii
//
// # References
// - https://github.com/takefumi-yoshii/nextjs-testing-strategy-2022/blob/main/src/tests/jest.customMatchers.tsx
// - https://zenn.dev/takepepe/articles/jest-custom-matcher-for-atomic-design
//
//
// # How to enable?
// To activate this customMatcher, we need to do the following
// 1. Put this file path to `setupFiles` in `vitest.config.ts
// 2. Write the signature of the test function we created in `global.d.ts` (ambient declaration)
//
import { getRoles } from '@testing-library/react';
import { expect } from 'vitest';

/** ref: [vitest/extending-matchers](https://vitest.dev/guide/extending-matchers) */
type ExpectationResult = {
  /** Pass or not? */
  pass: boolean;
  /** Err message */
  message: () => string;
  // If you pass these, they will automatically appear inside a diff when
  // the matcher does not pass, so you don't need to print the diff yourself
  actual?: unknown;
  expected?: unknown;
};

const groupRoles = ['group', 'article', 'list', 'term', 'tablist', 'tabpanel', 'table', 'rowgroup', 'row', 'combobox'];

const maybeLandmarkRoles = ['banner', 'contentinfo'];

const landmarkRoles = [
  'complementary',
  'form',
  // "main",
  'navigation',
  'region',
  'search',
];

const windowRoles = ['alertdialog', 'dialog'];

const ignoresRoles = ['generic', 'presentation'];

function includeGroupRole(keys: string[]) {
  return keys.map((key) => groupRoles.includes(key)).some(Boolean);
}

function includeMaybeLandmarkRole(keys: string[]) {
  return keys.map((key) => maybeLandmarkRoles.includes(key)).some(Boolean);
}

function includeLandmarkRole(keys: string[]) {
  return keys.map((key) => landmarkRoles.includes(key)).some(Boolean);
}

function includeWindowRole(keys: string[]) {
  return keys.map((key) => windowRoles.includes(key)).some(Boolean);
}

function includeMainRole(keys: string[]) {
  return keys.includes('main');
}

function getRoleKeys(container: HTMLElement) {
  return Object.keys(getRoles(container)).filter((key) => !ignoresRoles.includes(key));
}

function fail(message: string) {
  return { pass: false, message: () => message };
}

function toBeAtom(container: HTMLElement): ExpectationResult {
  const keys = getRoleKeys(container);
  if (keys.length >= 2) {
    return fail(
      `Atom should be structured by a single role, but got multiple roles: ${keys}. Consider simplifying the component's structure to a single role.`,
    );
  }
  if (includeGroupRole(keys)) {
    return fail(
      `Atom should not include group roles, but found: ${keys.filter((key) => groupRoles.includes(key))}. Try removing or restructuring these roles.`,
    );
  }
  if (includeWindowRole(keys)) {
    return fail(
      `Atom should not include window roles, but found: ${keys.filter((key) => windowRoles.includes(key))}. Make sure that this component does not represent a dialog or alert.`,
    );
  }
  if (includeLandmarkRole(keys) || includeMaybeLandmarkRole(keys) || includeMainRole(keys)) {
    return fail(
      `Atom should not include landmark roles like ${landmarkRoles}, but found: ${keys}. Review the semantic structure.`,
    );
  }
  return { pass: true, message: () => 'it is Atom' };
}

function toBeMolecule(container: HTMLElement): ExpectationResult {
  const keys = getRoleKeys(container);
  if (!(keys.length >= 2)) {
    return fail(
      `Molecule should be structured by multiple roles, but found only: ${keys}. Consider adding more semantic roles to this component.`,
    );
  }
  if (includeLandmarkRole(keys)) {
    return fail(
      `Molecule should not include landmark roles, but found: ${keys.filter((key) => landmarkRoles.includes(key))}. Avoid using roles like 'form' or 'navigation' in Molecules.`,
    );
  }
  if (includeWindowRole(keys)) {
    return fail(
      `Molecule should not include window roles, but found: ${keys.filter((key) => windowRoles.includes(key))}. Ensure that window-like roles such as 'dialog' are not present.`,
    );
  }
  if (includeMainRole(keys)) {
    return fail(
      `Molecule should not include the 'main' role, but found: 'main'. This role should be reserved for higher-level components like Templates.`,
    );
  }
  return { pass: true, message: () => 'it is Molecule' };
}

function toBeOrganism(container: HTMLElement): ExpectationResult {
  const keys = getRoleKeys(container);
  if (!(keys.length >= 2)) {
    return fail(
      `Organism should be structured by multiple roles, but found only: ${keys}. Consider adding more roles that define its structure.`,
    );
  }
  if (!(includeLandmarkRole(keys) || includeMaybeLandmarkRole(keys) || includeWindowRole(keys))) {
    return fail(
      `Organism should include landmark or window roles like 'form', 'navigation', 'region', or 'dialog', but found: ${keys}. Ensure the component contains meaningful structural roles.`,
    );
  }
  if (includeMainRole(keys)) {
    return fail(
      `Organism should not include the 'main' role, but found: 'main'. The 'main' role should be used in Templates.`,
    );
  }
  return { pass: true, message: () => 'it is Organism' };
}

function toBeTemplate(container: HTMLElement): ExpectationResult {
  const keys = getRoleKeys(container);
  if (!includeMainRole(keys)) {
    return fail(
      `Template should include the 'main' role, but found: ${keys}. Ensure that this component represents the main content area of the page.`,
    );
  }
  return { pass: true, message: () => 'it is Template' };
}

expect.extend({ toBeAtom, toBeMolecule, toBeOrganism, toBeTemplate });

import 'vitest';

/**
 * Custom matchers for extending Vitest assertions.
 * These matchers allow you to verify the structural roles of React components
 * based on the Atomic ReDesign methodology.
 *
 * @see https://vitest.dev/guide/extending-matchers
 */
interface CustomMatchers<R = unknown> {
  /**
   * Asserts that the component is an Atom.
   * Atoms are the smallest functional units in the Atomic ReDesign methodology.
   */
  toBeAtom: () => R;

  /**
   * Asserts that the component is a Molecule.
   * Molecules are groups of atoms that work together as a unit.
   */
  toBeMolecule: () => R;

  /**
   * Asserts that the component is an Organism.
   * Organisms are groups of molecules that form a distinct section of an interface.
   */
  toBeOrganism: () => R;

  /**
   * Asserts that the component is a Template.
   * Templates are the underlying structures that define the layout of a page.
   */
  toBeTemplate: () => R;
}

declare module 'vitest' {
  interface Assertion<T = unknown> extends CustomMatchers<T> {}
  interface AsymmetricMatchersContaining extends CustomMatchers {}
}

const fs = require('fs');
const path = require('path');

const INPUT_JSON = path.join(__dirname, 'Alternate_AnimationGroups.json');
const OUTPUT_SCHEMA = path.join(__dirname, 'schemas', 'fnis_to_oar_config.schema.json');

// --- JSON  ---
const rawData = fs.readFileSync(INPUT_JSON, 'utf-8');
/** @type {Record<string, string[]>} */
const data = JSON.parse(rawData);

// --- JSON Schema ---
const schema = {
  $schema: 'https://json-schema.org/draft/2020-12/schema',
  title: 'FnisToOarConfig',
  description: 'Detailed settings for converting FNIS alt anim to OAR can be configured.',
  type: 'object',
  properties: {
    $schema: {
      type: ['string', 'null'],
      description: 'JSON Schema version identifier.',
    },
    name: {
      type: ['string', 'null'],
      description:
        'Name for mod-specific OAR settings in `animations/OpenAnimationReplacer/<name>/config.json`. If unspecified, FNIS\'s namespace is used. e.g., "XPMSE".',
    },
    description: {
      type: ['string', 'null'],
      description: 'description for mod-specific OAR settings in `animations/OpenAnimationReplacer/<name>/config.json`',
    },
    author: {
      type: ['string', 'null'],
      description: 'author for mod-specific OAR settings in `animations/OpenAnimationReplacer/<name>/config.json`',
    },
    groups: {
      type: 'object',
      description: 'each `animations/OpenAnimationReplacer/<namespace>/<group name>_<slot>/config.json`',
      // see: https://github.com/GREsau/schemars/issues/424#issuecomment-2960508981
      propertyNames: {
        $ref: '#/$defs/GroupKind',
      },
      additionalProperties: {
        $ref: '#/$defs/GroupConfig',
      },
      default: {},
    },
  },
  additionalProperties: false,
  $defs: {
    GroupKind: {
      type: 'string',
      enum: Object.keys(data),
      description: 'Allowed FNIS alternate animation groups.',
    },
    GroupConfig: {
      type: 'object',
      description: 'Override options applied to a single FNIS alternate animation group.',
      additionalProperties: false,
      patternProperties: {
        '^\\d+$': {
          description: 'Number of FNIS <prefix><number>_<vanilla name>.hkx.',
          type: 'object',
          properties: {
            // Couldn't unique validation. see: https://stackoverflow.com/questions/65233788/unique-value-for-a-property-validation-using-json-schema
            rename_to: {
              type: 'string',
              description:
                'Optional animation rename for `animations/OpenAnimationReplacer/<namespace>/<rename_to>/config.json`(default: `group name>_<slot>`)\nNOTE: If not unique within groups, unintended overwrites will occur.',
            },
            description: {
              type: 'string',
              description: 'Optional description for this config.json',
            },
            priority: {
              type: ['integer', 'null'],
              minimum: 0,
              description: 'Optional priority for this config.json. Higher values take precedence in OAR.(default: 0)',
            },
            conditions: {
              type: 'array',
              description:
                'Arbitrary JSON array for OAR conditions. Unsafe: Does not check whether it is valid as an OAR.(default: [])',
              default: [],
            },
          },
          additionalProperties: false,
          default: {},
        },
        additionalProperties: false,
        default: {},
      },
    },
  },
};

fs.mkdirSync(path.dirname(OUTPUT_SCHEMA), { recursive: true });
fs.writeFileSync(OUTPUT_SCHEMA, JSON.stringify(schema, null, 2));
console.log(`JSON Schema generated: ${OUTPUT_SCHEMA}`);

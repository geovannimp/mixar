'use strict';

// Minimal, prototype-safe implementation of deepmerge v4's public API.
//
// Upstream deepmerge <= 4.3.1 has no patched release (GHSA-gcqx-w48m-w2h4),
// so style-dictionary's dependency on it is overridden with this module.
// Behaviour matches deepmerge v4 for the options style-dictionary uses:
// `isMergeableObject`, `arrayMerge` and `clone`.

const UNSAFE_KEYS = new Set(['__proto__', 'constructor', 'prototype']);

function isSpecial(value) {
  const tag = Object.prototype.toString.call(value);
  return tag === '[object RegExp]' || tag === '[object Date]';
}

function defaultIsMergeableObject(value) {
  return !!value && typeof value === 'object' && !isSpecial(value);
}

function isPlainObject(value) {
  if (Object.prototype.toString.call(value) !== '[object Object]') {
    return false;
  }
  const proto = Object.getPrototypeOf(value);
  return proto === null || proto === Object.prototype;
}

function propertyIsOnObject(object, property) {
  try {
    return property in object;
  } catch {
    return false;
  }
}

function propertyIsUnsafe(target, key) {
  if (UNSAFE_KEYS.has(key)) {
    return true;
  }
  return (
    propertyIsOnObject(target, key) &&
    !(
      Object.prototype.hasOwnProperty.call(target, key) &&
      Object.prototype.propertyIsEnumerable.call(target, key)
    )
  );
}

function getKeys(target) {
  return Object.keys(target);
}

function defaultArrayMerge(target, source, options) {
  return target.concat(source).map((element) => cloneUnlessOtherwiseSpecified(element, options));
}

function cloneUnlessOtherwiseSpecified(value, options) {
  return options.clone !== false && options.isMergeableObject(value)
    ? deepmerge(Array.isArray(value) ? [] : {}, value, options)
    : value;
}

function mergeObject(target, source, options) {
  const destination = {};

  if (options.isMergeableObject(target)) {
    for (const key of getKeys(target)) {
      if (UNSAFE_KEYS.has(key)) {
        continue;
      }
      defineKey(destination, key, cloneUnlessOtherwiseSpecified(target[key], options));
    }
  }

  for (const key of getKeys(source)) {
    if (propertyIsUnsafe(target, key)) {
      continue;
    }
    if (propertyIsOnObject(target, key) && options.isMergeableObject(source[key])) {
      defineKey(
        destination,
        key,
        deepmerge(target[key], source[key], options)
      );
    } else {
      defineKey(destination, key, cloneUnlessOtherwiseSpecified(source[key], options));
    }
  }

  return destination;
}

// Assignment to keys such as "__proto__" would mutate the prototype instead of
// creating an own property, so always define plain enumerable properties.
function defineKey(object, key, value) {
  Object.defineProperty(object, key, {
    value,
    enumerable: true,
    configurable: true,
    writable: true,
  });
}

function deepmerge(target, source, options) {
  options = options || {};
  options.arrayMerge = options.arrayMerge || defaultArrayMerge;
  options.isMergeableObject = options.isMergeableObject || defaultIsMergeableObject;

  const sourceIsArray = Array.isArray(source);
  const targetIsArray = Array.isArray(target);
  const sourceAndTargetTypesMatch = sourceIsArray === targetIsArray;

  if (!sourceAndTargetTypesMatch) {
    return cloneUnlessOtherwiseSpecified(source, options);
  }
  if (sourceIsArray) {
    return options.arrayMerge(target, source, options);
  }
  return mergeObject(target, source, options);
}

deepmerge.all = function all(objects, options) {
  if (!Array.isArray(objects)) {
    throw new Error('first argument should be an array');
  }
  return objects.reduce((prev, obj) => deepmerge(prev, obj, options), {});
};

module.exports = deepmerge;
module.exports.default = deepmerge;
module.exports.isPlainObject = isPlainObject;

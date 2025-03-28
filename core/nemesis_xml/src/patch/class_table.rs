/// key: className, value: (fieldName, fieldType)
pub type ClassTable = OrderedMap<&'static str, FieldInfo>;
pub type FieldInfo = OrderedMap<&'static str, &'static str>;
use phf::{phf_ordered_map, OrderedMap};
const CLASS_TABLE: ClassTable = phf_ordered_map! {
    "BGSGamebryoSequenceGenerator" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "pSequence"
    => "String", "eBlendModeFunction" => "String", "fPercent" => "F64", "events" =>
    "Array|Null", "fTime" => "F64", "bDelayedActivate" => "Bool", "bLooping" => "Bool",
    }, "BSBoneSwitchGenerator" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool",
    "pDefaultGenerator" => "Pointer", "ChildrenA" => "Array|Pointer", },
    "BSBoneSwitchGeneratorBoneData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "pGenerator" => "Pointer",
    "spBoneWeight" => "Pointer", }, "BSComputeAddBoneAnimModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "boneIndex" =>
    "I64", "translationLSOut" => "Object|Vector4", "rotationLSOut" =>
    "Object|Quaternion", "scaleLSOut" => "Object|Vector4", "pSkeletonMemory" =>
    "Pointer", }, "BSCyclicBlendTransitionGenerator" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "pBlenderGenerator" => "Pointer", "EventToFreezeBlendValue" =>
    "Object|hkbEventProperty", "EventToCrossBlend" => "Object|hkbEventProperty",
    "fBlendParameter" => "F64", "fTransitionDuration" => "F64", "eBlendCurve" =>
    "String", "pTransitionBlenderGenerator" => "Pointer", "pTransitionEffect" =>
    "Pointer", "currentMode" => "String", }, "BSDecomposeVectorModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "vector" => "Object|Vector4", "x" => "F64", "y" => "F64",
    "z" => "F64", "w" => "F64", }, "BSDirectAtModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "directAtTarget" =>
    "Bool", "sourceBoneIndex" => "I64", "startBoneIndex" => "I64", "endBoneIndex" =>
    "I64", "limitHeadingDegrees" => "F64", "limitPitchDegrees" => "F64",
    "offsetHeadingDegrees" => "F64", "offsetPitchDegrees" => "F64", "onGain" => "F64",
    "offGain" => "F64", "targetLocation" => "Object|Vector4", "userInfo" => "U64",
    "directAtCamera" => "Bool", "directAtCameraX" => "F64", "directAtCameraY" => "F64",
    "directAtCameraZ" => "F64", "active" => "Bool", "currentHeadingOffset" => "F64",
    "currentPitchOffset" => "F64", "timeStep" => "F64", "pSkeletonMemory" => "Pointer",
    "hasTarget" => "Bool", "directAtTargetLocation" => "Object|Vector4",
    "boneChainIndices" => "Array|Null", }, "BSDistTriggerModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "targetPosition" =>
    "Object|Vector4", "distance" => "F64", "distanceTrigger" => "F64", "triggerEvent" =>
    "Object|hkbEventProperty", }, "BSEventEveryNEventsModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "eventToCheckFor"
    => "Object|hkbEventProperty", "eventToSend" => "Object|hkbEventProperty",
    "numberOfEventsBeforeSend" => "I64", "minimumNumberOfEventsBeforeSend" => "I64",
    "randomizeNumberOfEvents" => "Bool", "numberOfEventsSeen" => "I64",
    "calculatedNumberOfEventsBeforeSend" => "I64", }, "BSEventOnDeactivateModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "event" => "Object|hkbEventProperty", },
    "BSEventOnFalseToTrueModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "bEnableEvent1" => "Bool", "bVariableToTest1" =>
    "Bool", "EventToSend1" => "Object|hkbEventProperty", "bEnableEvent2" => "Bool",
    "bVariableToTest2" => "Bool", "EventToSend2" => "Object|hkbEventProperty",
    "bEnableEvent3" => "Bool", "bVariableToTest3" => "Bool", "EventToSend3" =>
    "Object|hkbEventProperty", "bSlot1ActivatedLastFrame" => "Bool",
    "bSlot2ActivatedLastFrame" => "Bool", "bSlot3ActivatedLastFrame" => "Bool", },
    "BSGetTimeStepModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "timeStep" => "F64", }, "BSIStateManagerModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "iStateVar" => "I64", "stateData" =>
    "Array|Object|BSIStateManagerModifierBSiStateData", "myStateListener" =>
    "Object|BSIStateManagerModifierBSIStateManagerStateListener", },
    "BSIStateManagerModifierBSIStateManagerStateListener" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "pStateManager" => "Pointer",
    }, "BSIStateManagerModifierBSiStateData" => phf_ordered_map! { "pStateMachine" =>
    "Pointer", "StateID" => "I64", "iStateToSetAs" => "I64", }, "BSInterpValueModifier"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "source" => "F64", "target" => "F64", "result" => "F64",
    "gain" => "F64", "timeStep" => "F64", }, "BSIsActiveModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "bIsActive0" =>
    "Bool", "bInvertActive0" => "Bool", "bIsActive1" => "Bool", "bInvertActive1" =>
    "Bool", "bIsActive2" => "Bool", "bInvertActive2" => "Bool", "bIsActive3" => "Bool",
    "bInvertActive3" => "Bool", "bIsActive4" => "Bool", "bInvertActive4" => "Bool", },
    "BSLimbIKModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "limitAngleDegrees" => "F64", "currentAngle" => "F64",
    "startBoneIndex" => "I64", "endBoneIndex" => "I64", "gain" => "F64", "boneRadius" =>
    "F64", "castOffset" => "F64", "timeStep" => "F64", "pSkeletonMemory" => "Pointer", },
    "BSLookAtModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "lookAtTarget" => "Bool", "bones" =>
    "Array|Object|BSLookAtModifierBoneData", "eyeBones" =>
    "Array|Object|BSLookAtModifierBoneData", "limitAngleDegrees" => "F64",
    "limitAngleThresholdDegrees" => "F64", "continueLookOutsideOfLimit" => "Bool",
    "onGain" => "F64", "offGain" => "F64", "useBoneGains" => "Bool", "targetLocation" =>
    "Object|Vector4", "targetOutsideLimits" => "Bool", "targetOutOfLimitEvent" =>
    "Object|hkbEventProperty", "lookAtCamera" => "Bool", "lookAtCameraX" => "F64",
    "lookAtCameraY" => "F64", "lookAtCameraZ" => "F64", "timeStep" => "F64",
    "ballBonesValid" => "Bool", "pSkeletonMemory" => "Pointer", },
    "BSLookAtModifierBoneData" => phf_ordered_map! { "index" => "I64", "fwdAxisLS" =>
    "Object|Vector4", "limitAngleDegrees" => "F64", "onGain" => "F64", "offGain" =>
    "F64", "enabled" => "Bool", "currentFwdAxisLS" => "Object|Vector4", },
    "BSModifyOnceModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "pOnActivateModifier" => "Pointer",
    "pOnDeactivateModifier" => "Pointer", }, "BSOffsetAnimationGenerator" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "pDefaultGenerator" =>
    "Pointer", "pOffsetClipGenerator" => "Pointer", "fOffsetVariable" => "F64",
    "fOffsetRangeStart" => "F64", "fOffsetRangeEnd" => "F64", "BoneOffsetA" =>
    "Array|Null", "BoneIndexA" => "Array|Null", "fCurrentPercentage" => "F64",
    "iCurrentFrame" => "U64", "bZeroOffset" => "Bool", "bOffsetValid" => "Bool", },
    "BSPassByTargetTriggerModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "targetPosition" => "Object|Vector4", "radius" =>
    "F64", "movementDirection" => "Object|Vector4", "triggerEvent" =>
    "Object|hkbEventProperty", "targetPassed" => "Bool", },
    "BSRagdollContactListenerModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "contactEvent" => "Object|hkbEventProperty", "bones"
    => "Pointer", "throwEvent" => "Bool", "ragdollRigidBodies" => "Array|Pointer", },
    "BSSpeedSamplerModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "state" => "I64", "direction" => "F64", "goalSpeed"
    => "F64", "speedOut" => "F64", }, "BSSynchronizedClipGenerator" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "pClipGenerator" => "Pointer", "SyncAnimPrefix" => "String",
    "bSyncClipIgnoreMarkPlacement" => "Bool", "fGetToMarkTime" => "F64",
    "fMarkErrorThreshold" => "F64", "bLeadCharacter" => "Bool", "bReorientSupportChar" =>
    "Bool", "bApplyMotionFromRoot" => "Bool", "pSyncScene" => "Pointer", "StartMarkWS" =>
    "Object|QsTransform", "EndMarkWS" => "Object|QsTransform", "StartMarkMS" =>
    "Object|QsTransform", "fCurrentLerp" => "F64", "pLocalSyncBinding" => "Pointer",
    "pEventMap" => "Pointer", "sAnimationBindingIndex" => "I64", "bAtMark" => "Bool",
    "bAllCharactersInScene" => "Bool", "bAllCharactersAtMarks" => "Bool", },
    "BSTimerModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "alarmTimeSeconds" => "F64", "alarmEvent" =>
    "Object|hkbEventProperty", "resetAlarm" => "Bool", "secondsElapsed" => "F64", },
    "BSTweenerModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "tweenPosition" => "Bool", "tweenRotation" =>
    "Bool", "useTweenDuration" => "Bool", "tweenDuration" => "F64", "targetPosition" =>
    "Object|Vector4", "targetRotation" => "Object|Quaternion", "duration" => "F64",
    "startTransform" => "Object|QsTransform", "time" => "F64", },
    "BSiStateTaggingGenerator" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool",
    "pDefaultGenerator" => "Pointer", "iStateToSetAs" => "I64", "iPriority" => "I64", },
    "Matrix3" => phf_ordered_map! { "x" => "Object|Vector4", "y" => "Object|Vector4", "z"
    => "Object|Vector4", }, "Matrix4" => phf_ordered_map! { "x" => "Object|Vector4", "y"
    => "Object|Vector4", "z" => "Object|Vector4", "w" => "Object|Vector4", },
    "QsTransform" => phf_ordered_map! { "transition" => "Object|Vector4", "quaternion" =>
    "Object|Vector4", "scale" => "Object|Vector4", }, "Quaternion" => phf_ordered_map! {
    "x" => "F64", "y" => "F64", "z" => "F64", "scaler" => "F64", }, "Rotation" =>
    phf_ordered_map! { "x" => "Object|Vector4", "y" => "Object|Vector4", "z" =>
    "Object|Vector4", }, "Transform" => phf_ordered_map! { "Rotation" =>
    "Object|Vector4", "transition" => "Object|Vector4", }, "Variant" => phf_ordered_map!
    { "object" => "Pointer", "class" => "Pointer", }, "Vector4" => phf_ordered_map! { "x"
    => "F64", "y" => "F64", "z" => "F64", "w" => "F64", }, "hkAabb" => phf_ordered_map! {
    "min" => "Object|Vector4", "max" => "Object|Vector4", }, "hkAabbHalf" =>
    phf_ordered_map! { "data" => "U64", "extras" => "U64", }, "hkAabbUint32" =>
    phf_ordered_map! { "min" => "U64", "expansionMin" => "U64", "expansionShift" =>
    "U64", "max" => "U64", "expansionMax" => "U64", "shapeKeyByte" => "U64", },
    "hkAlignSceneToNodeOptions" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "invert" => "Bool", "transformPositionX" => "Bool",
    "transformPositionY" => "Bool", "transformPositionZ" => "Bool", "transformRotation"
    => "Bool", "transformScale" => "Bool", "transformSkew" => "Bool", "keyframe" =>
    "I64", "nodeName" => "String", }, "hkArrayTypeAttribute" => phf_ordered_map! { "type"
    => "String", }, "hkBaseObject" => phf_ordered_map! {}, "hkBitField" =>
    phf_ordered_map! { "words" => "Array|U64", "numBits" => "I64", }, "hkClass" =>
    phf_ordered_map! { "name" => "String", "parent" => "Pointer", "objectSize" => "I64",
    "numImplementedInterfaces" => "I64", "declaredEnums" => "Array|Object|hkClassEnum",
    "declaredMembers" => "Array|Object|hkClassMember", "defaults" => "Pointer",
    "attributes" => "Pointer", "flags" => "String", "describedVersion" => "I64", },
    "hkClassEnum" => phf_ordered_map! { "name" => "String", "items" =>
    "Array|Object|hkClassEnumItem", "attributes" => "Pointer", "flags" => "String", },
    "hkClassEnumItem" => phf_ordered_map! { "value" => "I64", "name" => "String", },
    "hkClassMember" => phf_ordered_map! { "name" => "String", "class" => "Pointer",
    "enum" => "Pointer", "type" => "String", "subtype" => "String", "cArraySize" =>
    "I64", "flags" => "String", "offset" => "U64", "attributes" => "Pointer", },
    "hkColor" => phf_ordered_map! {}, "hkContactPoint" => phf_ordered_map! { "position"
    => "Object|Vector4", "separatingNormal" => "Object|Vector4", },
    "hkContactPointMaterial" => phf_ordered_map! { "userData" => "U64", "friction" =>
    "U64", "restitution" => "U64", "maxImpulse" => "U64", "flags" => "U64", },
    "hkCustomAttributes" => phf_ordered_map! { "attributes" =>
    "Array|Object|hkCustomAttributesAttribute", }, "hkCustomAttributesAttribute" =>
    phf_ordered_map! { "name" => "String", "value" => "Object|Variant", },
    "hkDataObjectTypeAttribute" => phf_ordered_map! { "typeName" => "String", },
    "hkDescriptionAttribute" => phf_ordered_map! { "string" => "String", },
    "hkDocumentationAttribute" => phf_ordered_map! { "docsSectionTag" => "String", },
    "hkGeometry" => phf_ordered_map! { "vertices" => "Array|Object|Vector4", "triangles"
    => "Array|Object|hkGeometryTriangle", }, "hkGeometryTriangle" => phf_ordered_map! {
    "a" => "I64", "b" => "I64", "c" => "I64", "material" => "I64", }, "hkGizmoAttribute"
    => phf_ordered_map! { "visible" => "Bool", "label" => "String", "type" => "String",
    }, "hkHalf8" => phf_ordered_map! { "quad" => "F64", }, "hkIndexedTransformSet" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "matrices"
    => "Array|Object|Matrix4", "inverseMatrices" => "Array|Object|Matrix4",
    "matricesOrder" => "Array|I64", "matricesNames" => "Array|String", "indexMappings" =>
    "Array|Object|hkMeshBoneIndexMapping", "allMatricesAreAffine" => "Bool", },
    "hkLinkAttribute" => phf_ordered_map! { "type" => "String", }, "hkLocalFrame" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", },
    "hkLocalFrameGroup" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "name" => "String", }, "hkMemoryMeshBody" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "transform"
    => "Object|Matrix4", "transformSet" => "Pointer", "shape" => "Pointer",
    "vertexBuffers" => "Array|Pointer", "name" => "String", }, "hkMemoryMeshMaterial" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "materialName" => "String", "textures" => "Array|Pointer", }, "hkMemoryMeshShape" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "sections"
    => "Array|Object|hkMeshSectionCinfo", "indices16" => "Array|U64", "indices32" =>
    "Array|U64", "name" => "String", }, "hkMemoryMeshTexture" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "filename" => "String", "data"
    => "Array|U64", "format" => "String", "hasMipMaps" => "Bool", "filterMode" =>
    "String", "usageHint" => "String", "textureCoordChannel" => "I64", },
    "hkMemoryMeshVertexBuffer" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "format" => "Object|hkVertexFormat", "elementOffsets" =>
    "I64", "memory" => "Array|U64", "vertexStride" => "I64", "locked" => "Bool",
    "numVertices" => "I64", "isBigEndian" => "Bool", "isSharable" => "Bool", },
    "hkMemoryResourceContainer" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "name" => "String", "parent" => "Pointer",
    "resourceHandles" => "Array|Pointer", "children" => "Array|Pointer", },
    "hkMemoryResourceHandle" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variant" => "Pointer", "name" => "String", "references"
    => "Array|Object|hkMemoryResourceHandleExternalLink", },
    "hkMemoryResourceHandleExternalLink" => phf_ordered_map! { "memberName" => "String",
    "externalId" => "String", }, "hkMemoryTrackerAttribute" => phf_ordered_map! {},
    "hkMeshBody" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" =>
    "I64", }, "hkMeshBoneIndexMapping" => phf_ordered_map! { "mapping" => "Array|I64", },
    "hkMeshMaterial" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", }, "hkMeshSection" => phf_ordered_map! { "primitiveType" => "String",
    "numPrimitives" => "I64", "numIndices" => "I64", "vertexStartIndex" => "I64",
    "transformIndex" => "I64", "indexType" => "String", "indices" => "Pointer",
    "vertexBuffer" => "Pointer", "material" => "Pointer", "sectionIndex" => "I64", },
    "hkMeshSectionCinfo" => phf_ordered_map! { "vertexBuffer" => "Pointer", "material" =>
    "Pointer", "primitiveType" => "String", "numPrimitives" => "I64", "indexType" =>
    "String", "indices" => "Pointer", "vertexStartIndex" => "I64", "transformIndex" =>
    "I64", }, "hkMeshShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkMeshTexture" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", }, "hkMeshVertexBuffer" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", },
    "hkModelerNodeTypeAttribute" => phf_ordered_map! { "type" => "String", },
    "hkMonitorStreamColorTable" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "colorPairs" =>
    "Array|Object|hkMonitorStreamColorTableColorPair", "defaultColor" => "U64", },
    "hkMonitorStreamColorTableColorPair" => phf_ordered_map! { "colorName" => "String",
    "color" => "String", }, "hkMonitorStreamFrameInfo" => phf_ordered_map! { "heading" =>
    "String", "indexOfTimer0" => "I64", "indexOfTimer1" => "I64", "absoluteTimeCounter"
    => "String", "timerFactor0" => "F64", "timerFactor1" => "F64", "threadId" => "I64",
    "frameStreamStart" => "I64", "frameStreamEnd" => "I64", }, "hkMonitorStreamStringMap"
    => phf_ordered_map! { "map" => "Array|Object|hkMonitorStreamStringMapStringMap", },
    "hkMonitorStreamStringMapStringMap" => phf_ordered_map! { "id" => "U64", "string" =>
    "String", }, "hkMoppBvTreeShapeBase" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "userData" => "U64", "type" => "String",
    "bvTreeType" => "String", "code" => "Pointer", "moppData" => "Pointer",
    "moppDataSize" => "U64", "codeInfoCopy" => "Object|Vector4", }, "hkMotionState" =>
    phf_ordered_map! { "transform" => "Object|Transform", "sweptTransform" =>
    "Object|hkSweptTransform", "deltaAngle" => "Object|Vector4", "objectRadius" => "F64",
    "linearDamping" => "F64", "angularDamping" => "F64", "timeFactor" => "F64",
    "maxLinearVelocity" => "U64", "maxAngularVelocity" => "U64", "deactivationClass" =>
    "U64", }, "hkMultiThreadCheck" => phf_ordered_map! { "threadId" => "U64",
    "stackTraceId" => "I64", "markCount" => "U64", "markBitStack" => "U64", },
    "hkMultipleVertexBuffer" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "vertexFormat" => "Object|hkVertexFormat",
    "lockedElements" => "Array|Object|hkMultipleVertexBufferLockedElement",
    "lockedBuffer" => "Pointer", "elementInfos" =>
    "Array|Object|hkMultipleVertexBufferElementInfo", "vertexBufferInfos" =>
    "Array|Object|hkMultipleVertexBufferVertexBufferInfo", "numVertices" => "I64",
    "isLocked" => "Bool", "updateCount" => "U64", "writeLock" => "Bool", "isSharable" =>
    "Bool", "constructionComplete" => "Bool", }, "hkMultipleVertexBufferElementInfo" =>
    phf_ordered_map! { "vertexBufferIndex" => "U64", "elementIndex" => "U64", },
    "hkMultipleVertexBufferLockedElement" => phf_ordered_map! { "vertexBufferIndex" =>
    "U64", "elementIndex" => "U64", "lockedBufferIndex" => "U64", "vertexFormatIndex" =>
    "U64", "lockFlags" => "U64", "outputBufferIndex" => "U64", "emulatedIndex" => "I64",
    }, "hkMultipleVertexBufferVertexBufferInfo" => phf_ordered_map! { "vertexBuffer" =>
    "Pointer", "lockedVertices" => "Pointer", "isLocked" => "Bool", }, "hkPackedVector3"
    => phf_ordered_map! { "values" => "I64", }, "hkPackfileHeader" => phf_ordered_map! {
    "magic" => "I64", "userTag" => "I64", "fileVersion" => "I64", "layoutRules" => "U64",
    "numSections" => "I64", "contentsSectionIndex" => "I64", "contentsSectionOffset" =>
    "I64", "contentsClassNameSectionIndex" => "I64", "contentsClassNameSectionOffset" =>
    "I64", "contentsVersion" => "String", "flags" => "I64", "pad" => "I64", },
    "hkPackfileSectionHeader" => phf_ordered_map! { "sectionTag" => "String", "nullByte"
    => "String", "absoluteDataStart" => "I64", "localFixupsOffset" => "I64",
    "globalFixupsOffset" => "I64", "virtualFixupsOffset" => "I64", "exportsOffset" =>
    "I64", "importsOffset" => "I64", "endOffset" => "I64", }, "hkPostFinishAttribute" =>
    phf_ordered_map! { "postFinishFunction" => "Pointer", }, "hkQTransform" =>
    phf_ordered_map! { "rotation" => "Object|Quaternion", "translation" =>
    "Object|Vector4", }, "hkRangeInt32Attribute" => phf_ordered_map! { "absmin" => "I64",
    "absmax" => "I64", "softmin" => "I64", "softmax" => "I64", }, "hkRangeRealAttribute"
    => phf_ordered_map! { "absmin" => "F64", "absmax" => "F64", "softmin" => "F64",
    "softmax" => "F64", }, "hkReferencedObject" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", }, "hkReflectedFileAttribute" =>
    phf_ordered_map! { "value" => "String", }, "hkResourceBase" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", }, "hkResourceContainer" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", },
    "hkResourceHandle" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", }, "hkRootLevelContainer" => phf_ordered_map! { "namedVariants" =>
    "Array|Object|hkRootLevelContainerNamedVariant", },
    "hkRootLevelContainerNamedVariant" => phf_ordered_map! { "name" => "String",
    "className" => "String", "variant" => "Pointer", }, "hkSemanticsAttribute" =>
    phf_ordered_map! { "type" => "String", }, "hkSimpleLocalFrame" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "transform" =>
    "Object|Transform", "children" => "Array|Pointer", "parentFrame" => "Pointer",
    "group" => "Pointer", "name" => "String", }, "hkSphere" => phf_ordered_map! { "pos"
    => "Object|Vector4", }, "hkSweptTransform" => phf_ordered_map! { "centerOfMass0" =>
    "Object|Vector4", "centerOfMass1" => "Object|Vector4", "rotation0" =>
    "Object|Quaternion", "rotation1" => "Object|Quaternion", "centerOfMassLocal" =>
    "Object|Vector4", }, "hkTraceStreamTitle" => phf_ordered_map! { "value" => "String",
    }, "hkTrackerSerializableScanSnapshot" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "allocations" =>
    "Array|Object|hkTrackerSerializableScanSnapshotAllocation", "blocks" =>
    "Array|Object|hkTrackerSerializableScanSnapshotBlock", "refs" => "Array|I64",
    "typeNames" => "Array|U64", "traceText" => "Array|U64", "traceAddrs" => "Array|U64",
    "traceParents" => "Array|I64", }, "hkTrackerSerializableScanSnapshotAllocation" =>
    phf_ordered_map! { "start" => "U64", "size" => "U64", "traceId" => "I64", },
    "hkTrackerSerializableScanSnapshotBlock" => phf_ordered_map! { "typeIndex" => "I64",
    "start" => "U64", "size" => "U64", "arraySize" => "I64", "startReferenceIndex" =>
    "I64", "numReferences" => "I64", }, "hkUiAttribute" => phf_ordered_map! { "visible"
    => "Bool", "hideInModeler" => "String", "label" => "String", "group" => "String",
    "hideBaseClassMembers" => "String", "endGroup" => "Bool", "endGroup2" => "Bool",
    "advanced" => "Bool", }, "hkVertexFormat" => phf_ordered_map! { "elements" =>
    "Object|hkVertexFormatElement", "numElements" => "I64", }, "hkVertexFormatElement" =>
    phf_ordered_map! { "dataType" => "String", "numValues" => "U64", "usage" => "String",
    "subUsage" => "U64", "flags" => "String", "pad" => "U64", },
    "hkWorldMemoryAvailableWatchDog" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkaAnimatedReferenceFrame" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", }, "hkaAnimation" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" =>
    "String", "duration" => "F64", "numberOfTransformTracks" => "I64",
    "numberOfFloatTracks" => "I64", "extractedMotion" => "Pointer", "annotationTracks" =>
    "Array|Object|hkaAnnotationTrack", }, "hkaAnimationBinding" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "originalSkeletonName" =>
    "String", "animation" => "Pointer", "transformTrackToBoneIndices" => "Array|I64",
    "floatTrackToFloatSlotIndices" => "Array|I64", "blendHint" => "String", },
    "hkaAnimationContainer" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "skeletons" => "Array|Pointer", "animations" =>
    "Array|Pointer", "bindings" => "Array|Pointer", "attachments" => "Array|Pointer",
    "skins" => "Array|Pointer", }, "hkaAnimationPreviewColorContainer" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "previewColor" => "Array|U64", }, "hkaAnnotationTrack" => phf_ordered_map! {
    "trackName" => "String", "annotations" =>
    "Array|Object|hkaAnnotationTrackAnnotation", }, "hkaAnnotationTrackAnnotation" =>
    phf_ordered_map! { "time" => "F64", "text" => "String", }, "hkaBone" =>
    phf_ordered_map! { "name" => "String", "lockTranslation" => "Bool", },
    "hkaBoneAttachment" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "originalSkeletonName" => "String", "boneFromAttachment"
    => "Object|Matrix4", "attachment" => "Pointer", "name" => "String", "boneIndex" =>
    "I64", }, "hkaDefaultAnimatedReferenceFrame" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "up" => "Object|Vector4", "forward" =>
    "Object|Vector4", "duration" => "F64", "referenceFrameSamples" =>
    "Array|Object|Vector4", }, "hkaDeltaCompressedAnimation" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" => "String", "duration"
    => "F64", "numberOfTransformTracks" => "I64", "numberOfFloatTracks" => "I64",
    "extractedMotion" => "Pointer", "annotationTracks" =>
    "Array|Object|hkaAnnotationTrack", "numberOfPoses" => "I64", "blockSize" => "I64",
    "qFormat" => "Object|hkaDeltaCompressedAnimationQuantizationFormat",
    "quantizedDataIdx" => "U64", "quantizedDataSize" => "U64", "staticMaskIdx" => "U64",
    "staticMaskSize" => "U64", "staticDOFsIdx" => "U64", "staticDOFsSize" => "U64",
    "numStaticTransformDOFs" => "U64", "numDynamicTransformDOFs" => "U64",
    "totalBlockSize" => "U64", "lastBlockSize" => "U64", "dataBuffer" => "Array|U64", },
    "hkaDeltaCompressedAnimationQuantizationFormat" => phf_ordered_map! { "maxBitWidth"
    => "U64", "preserved" => "U64", "numD" => "U64", "offsetIdx" => "U64", "scaleIdx" =>
    "U64", "bitWidthIdx" => "U64", }, "hkaFootstepAnalysisInfo" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "name" => "Array|String",
    "nameStrike" => "Array|String", "nameLift" => "Array|String", "nameLock" =>
    "Array|String", "nameUnlock" => "Array|String", "minPos" => "Array|F64", "maxPos" =>
    "Array|F64", "minVel" => "Array|F64", "maxVel" => "Array|F64", "allBonesDown" =>
    "Array|F64", "anyBonesDown" => "Array|F64", "posTol" => "F64", "velTol" => "F64",
    "duration" => "F64", }, "hkaFootstepAnalysisInfoContainer" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "previewInfo" =>
    "Array|Pointer", }, "hkaInterleavedUncompressedAnimation" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" => "String", "duration"
    => "F64", "numberOfTransformTracks" => "I64", "numberOfFloatTracks" => "I64",
    "extractedMotion" => "Pointer", "annotationTracks" =>
    "Array|Object|hkaAnnotationTrack", "transforms" => "Array|Object|QsTransform",
    "floats" => "Array|F64", }, "hkaKeyFrameHierarchyUtility" => phf_ordered_map! {},
    "hkaKeyFrameHierarchyUtilityControlData" => phf_ordered_map! { "hierarchyGain" =>
    "F64", "velocityDamping" => "F64", "accelerationGain" => "F64", "velocityGain" =>
    "F64", "positionGain" => "F64", "positionMaxLinearVelocity" => "F64",
    "positionMaxAngularVelocity" => "F64", "snapGain" => "F64", "snapMaxLinearVelocity"
    => "F64", "snapMaxAngularVelocity" => "F64", "snapMaxLinearDistance" => "F64",
    "snapMaxAngularDistance" => "F64", }, "hkaMeshBinding" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "mesh" => "Pointer",
    "originalSkeletonName" => "String", "skeleton" => "Pointer", "mappings" =>
    "Array|Object|hkaMeshBindingMapping", "boneFromSkinMeshTransforms" =>
    "Array|Object|Transform", }, "hkaMeshBindingMapping" => phf_ordered_map! { "mapping"
    => "Array|I64", }, "hkaQuantizedAnimation" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "type" => "String", "duration" => "F64",
    "numberOfTransformTracks" => "I64", "numberOfFloatTracks" => "I64", "extractedMotion"
    => "Pointer", "annotationTracks" => "Array|Object|hkaAnnotationTrack", "data" =>
    "Array|U64", "endian" => "U64", "skeleton" => "Pointer", },
    "hkaQuantizedAnimationTrackCompressionParams" => phf_ordered_map! {
    "rotationTolerance" => "F64", "translationTolerance" => "F64", "scaleTolerance" =>
    "F64", "floatingTolerance" => "F64", }, "hkaRagdollInstance" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "rigidBodies" =>
    "Array|Pointer", "constraints" => "Array|Pointer", "boneToRigidBodyMap" =>
    "Array|I64", "skeleton" => "Pointer", }, "hkaSkeleton" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "name" => "String",
    "parentIndices" => "Array|I64", "bones" => "Array|Object|hkaBone", "referencePose" =>
    "Array|Object|QsTransform", "referenceFloats" => "Array|F64", "floatSlots" =>
    "Array|String", "localFrames" => "Array|Object|hkaSkeletonLocalFrameOnBone", },
    "hkaSkeletonLocalFrameOnBone" => phf_ordered_map! { "localFrame" => "Pointer",
    "boneIndex" => "I64", }, "hkaSkeletonMapper" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "mapping" => "Object|hkaSkeletonMapperData", },
    "hkaSkeletonMapperData" => phf_ordered_map! { "skeletonA" => "Pointer", "skeletonB"
    => "Pointer", "simpleMappings" => "Array|Object|hkaSkeletonMapperDataSimpleMapping",
    "chainMappings" => "Array|Object|hkaSkeletonMapperDataChainMapping", "unmappedBones"
    => "Array|I64", "extractedMotionMapping" => "Object|QsTransform", "keepUnmappedLocal"
    => "Bool", "mappingType" => "String", }, "hkaSkeletonMapperDataChainMapping" =>
    phf_ordered_map! { "startBoneA" => "I64", "endBoneA" => "I64", "startBoneB" => "I64",
    "endBoneB" => "I64", "startAFromBTransform" => "Object|QsTransform",
    "endAFromBTransform" => "Object|QsTransform", }, "hkaSkeletonMapperDataSimpleMapping"
    => phf_ordered_map! { "boneA" => "I64", "boneB" => "I64", "aFromBTransform" =>
    "Object|QsTransform", }, "hkaSplineCompressedAnimation" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" => "String", "duration"
    => "F64", "numberOfTransformTracks" => "I64", "numberOfFloatTracks" => "I64",
    "extractedMotion" => "Pointer", "annotationTracks" =>
    "Array|Object|hkaAnnotationTrack", "numFrames" => "I64", "numBlocks" => "I64",
    "maxFramesPerBlock" => "I64", "maskAndQuantizationSize" => "I64", "blockDuration" =>
    "F64", "blockInverseDuration" => "F64", "frameDuration" => "F64", "blockOffsets" =>
    "Array|U64", "floatBlockOffsets" => "Array|U64", "transformOffsets" => "Array|U64",
    "floatOffsets" => "Array|U64", "data" => "Array|U64", "endian" => "I64", },
    "hkaSplineCompressedAnimationAnimationCompressionParams" => phf_ordered_map! {
    "maxFramesPerBlock" => "U64", "enableSampleSingleTracks" => "Bool", },
    "hkaSplineCompressedAnimationTrackCompressionParams" => phf_ordered_map! {
    "rotationTolerance" => "F64", "translationTolerance" => "F64", "scaleTolerance" =>
    "F64", "floatingTolerance" => "F64", "rotationDegree" => "U64", "translationDegree"
    => "U64", "scaleDegree" => "U64", "floatingDegree" => "U64",
    "rotationQuantizationType" => "String", "translationQuantizationType" => "String",
    "scaleQuantizationType" => "String", "floatQuantizationType" => "String", },
    "hkaWaveletCompressedAnimation" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "type" => "String", "duration" => "F64",
    "numberOfTransformTracks" => "I64", "numberOfFloatTracks" => "I64", "extractedMotion"
    => "Pointer", "annotationTracks" => "Array|Object|hkaAnnotationTrack",
    "numberOfPoses" => "I64", "blockSize" => "I64", "qFormat" =>
    "Object|hkaWaveletCompressedAnimationQuantizationFormat", "staticMaskIdx" => "U64",
    "staticDOFsIdx" => "U64", "numStaticTransformDOFs" => "U64",
    "numDynamicTransformDOFs" => "U64", "blockIndexIdx" => "U64", "blockIndexSize" =>
    "U64", "quantizedDataIdx" => "U64", "quantizedDataSize" => "U64", "dataBuffer" =>
    "Array|U64", }, "hkaWaveletCompressedAnimationCompressionParams" => phf_ordered_map!
    { "quantizationBits" => "U64", "blockSize" => "U64", "preserve" => "U64", "truncProp"
    => "F64", "useOldStyleTruncation" => "Bool", "absolutePositionTolerance" => "F64",
    "relativePositionTolerance" => "F64", "rotationTolerance" => "F64", "scaleTolerance"
    => "F64", "absoluteFloatTolerance" => "F64", },
    "hkaWaveletCompressedAnimationQuantizationFormat" => phf_ordered_map! { "maxBitWidth"
    => "U64", "preserved" => "U64", "numD" => "U64", "offsetIdx" => "U64", "scaleIdx" =>
    "U64", "bitWidthIdx" => "U64", }, "hkbAttachmentModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool",
    "sendToAttacherOnAttach" => "Object|hkbEventProperty", "sendToAttacheeOnAttach" =>
    "Object|hkbEventProperty", "sendToAttacherOnDetach" => "Object|hkbEventProperty",
    "sendToAttacheeOnDetach" => "Object|hkbEventProperty", "attachmentSetup" =>
    "Pointer", "attacherHandle" => "Pointer", "attacheeHandle" => "Pointer",
    "attacheeLayer" => "I64", "attacheeRB" => "Pointer", "oldMotionType" => "String",
    "oldFilterInfo" => "I64", "attachment" => "Pointer", }, "hkbAttachmentSetup" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "blendInTime" => "F64", "moveAttacherFraction" => "F64", "gain" => "F64",
    "extrapolationTimeStep" => "F64", "fixUpGain" => "F64", "maxLinearDistance" => "F64",
    "maxAngularDistance" => "F64", "attachmentType" => "String", },
    "hkbAttributeModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "assignments" =>
    "Array|Object|hkbAttributeModifierAssignment", }, "hkbAttributeModifierAssignment" =>
    phf_ordered_map! { "attributeIndex" => "I64", "attributeValue" => "F64", },
    "hkbAuxiliaryNodeInfo" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "type" => "String", "depth" => "U64",
    "referenceBehaviorName" => "String", "selfTransitionNames" => "Array|String", },
    "hkbBehaviorEventsInfo" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "characterId" => "U64", "externalEventIds" => "Array|I64",
    "padding" => "I64", }, "hkbBehaviorGraph" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "variableBindingSet" => "Pointer",
    "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool", "userData" =>
    "U64", "name" => "String", "id" => "I64", "cloneState" => "String", "padNode" =>
    "Bool", "variableMode" => "String", "uniqueIdPool" => "Array|Null",
    "idToStateMachineTemplateMap" => "Pointer", "mirroredExternalIdMap" => "Array|Null",
    "pseudoRandomGenerator" => "Pointer", "rootGenerator" => "Pointer", "data" =>
    "Pointer", "rootGeneratorClone" => "Pointer", "activeNodes" => "Pointer",
    "activeNodeTemplateToIndexMap" => "Pointer", "activeNodesChildrenIndices" =>
    "Pointer", "globalTransitionData" => "Pointer", "eventIdMap" => "Pointer",
    "attributeIdMap" => "Pointer", "variableIdMap" => "Pointer", "characterPropertyIdMap"
    => "Pointer", "variableValueSet" => "Pointer", "nodeTemplateToCloneMap" => "Pointer",
    "nodeCloneToTemplateMap" => "Pointer", "stateListenerTemplateToCloneMap" =>
    "Pointer", "nodePartitionInfo" => "Pointer", "numIntermediateOutputs" => "I64",
    "jobs" => "Array|Pointer", "allPartitionMemory" => "Array|Pointer", "numStaticNodes"
    => "I64", "nextUniqueId" => "I64", "isActive" => "Bool", "isLinked" => "Bool",
    "updateActiveNodes" => "Bool", "stateOrTransitionChanged" => "Bool", },
    "hkbBehaviorGraphData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "attributeDefaults" => "Array|F64", "variableInfos" =>
    "Array|Object|hkbVariableInfo", "characterPropertyInfos" =>
    "Array|Object|hkbVariableInfo", "eventInfos" => "Array|Object|hkbEventInfo",
    "wordMinVariableValues" => "Array|Object|hkbVariableValue", "wordMaxVariableValues"
    => "Array|Object|hkbVariableValue", "variableInitialValues" => "Pointer",
    "stringData" => "Pointer", }, "hkbBehaviorGraphInternalState" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "nodeInternalStateInfos" =>
    "Array|Pointer", "variableValueSet" => "Pointer", },
    "hkbBehaviorGraphInternalStateInfo" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "characterId" => "U64", "internalState" => "Pointer",
    "auxiliaryNodeInfo" => "Array|Pointer", "activeEventIds" => "Array|I64",
    "activeVariableIds" => "Array|I64", }, "hkbBehaviorGraphStringData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "eventNames" => "Array|String", "attributeNames" => "Array|String", "variableNames"
    => "Array|String", "characterPropertyNames" => "Array|String", }, "hkbBehaviorInfo"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "characterId" => "U64", "data" => "Pointer", "idToNamePairs" =>
    "Array|Object|hkbBehaviorInfoIdToNamePair", }, "hkbBehaviorInfoIdToNamePair" =>
    phf_ordered_map! { "behaviorName" => "String", "nodeName" => "String", "toolType" =>
    "String", "id" => "I64", }, "hkbBehaviorReferenceGenerator" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "behaviorName" => "String", "behavior" => "Pointer", },
    "hkbBindable" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" =>
    "I64", "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", }, "hkbBlendCurveUtils" => phf_ordered_map! {},
    "hkbBlenderGenerator" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool",
    "referencePoseWeightThreshold" => "F64", "blendParameter" => "F64",
    "minCyclicBlendParameter" => "F64", "maxCyclicBlendParameter" => "F64",
    "indexOfSyncMasterChild" => "I64", "flags" => "I64", "subtractLastChild" => "Bool",
    "children" => "Array|Pointer", "childrenInternalStates" => "Array|Null",
    "sortedChildren" => "Array|Null", "endIntervalWeight" => "F64", "numActiveChildren"
    => "I64", "beginIntervalIndex" => "I64", "endIntervalIndex" => "I64", "initSync" =>
    "Bool", "doSubtractiveBlend" => "Bool", }, "hkbBlenderGeneratorChild" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "generator" => "Pointer", "boneWeights" => "Pointer",
    "weight" => "F64", "worldFromModelWeight" => "F64", },
    "hkbBlenderGeneratorChildInternalState" => phf_ordered_map! { "isActive" => "Bool",
    "syncNextFrame" => "Bool", }, "hkbBlenderGeneratorInternalState" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "childrenInternalStates" =>
    "Array|Object|hkbBlenderGeneratorChildInternalState", "sortedChildren" =>
    "Array|I64", "endIntervalWeight" => "F64", "numActiveChildren" => "I64",
    "beginIntervalIndex" => "I64", "endIntervalIndex" => "I64", "initSync" => "Bool",
    "doSubtractiveBlend" => "Bool", }, "hkbBlendingTransitionEffect" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "selfTransitionMode" => "String", "eventMode" => "String",
    "defaultEventMode" => "String", "duration" => "F64", "toGeneratorStartTimeFraction"
    => "F64", "flags" => "String", "endMode" => "String", "blendCurve" => "String",
    "fromGenerator" => "Pointer", "toGenerator" => "Pointer",
    "characterPoseAtBeginningOfTransition" => "Array|Null", "timeRemaining" => "F64",
    "timeInTransition" => "F64", "applySelfTransition" => "Bool",
    "initializeCharacterPose" => "Bool", }, "hkbBlendingTransitionEffectInternalState" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "characterPoseAtBeginningOfTransition" => "Array|Object|QsTransform", "timeRemaining"
    => "F64", "timeInTransition" => "F64", "applySelfTransition" => "Bool",
    "initializeCharacterPose" => "Bool", }, "hkbBoneIndexArray" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "boneIndices" => "Array|I64", }, "hkbBoneWeightArray" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "boneWeights" => "Array|F64", }, "hkbBoolVariableSequencedData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "samples" =>
    "Array|Object|hkbBoolVariableSequencedDataSample", "variableIndex" => "I64", },
    "hkbBoolVariableSequencedDataSample" => phf_ordered_map! { "time" => "F64", "value"
    => "Bool", }, "hkbCameraShakeEventPayload" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "amplitude" => "F64", "halfLife" => "F64", },
    "hkbCharacter" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" =>
    "I64", "nearbyCharacters" => "Array|Pointer", "currentLod" => "I64", "numTracksInLod"
    => "I64", "name" => "String", "ragdollDriver" => "Pointer",
    "characterControllerDriver" => "Pointer", "footIkDriver" => "Pointer", "handIkDriver"
    => "Pointer", "setup" => "Pointer", "behaviorGraph" => "Pointer", "projectData" =>
    "Pointer", "animationBindingSet" => "Pointer", "raycastInterface" => "Pointer",
    "world" => "Pointer", "eventQueue" => "Pointer", "worldFromModel" => "Pointer",
    "poseLocal" => "Array|Null", "deleteWorldFromModel" => "Bool", "deletePoseLocal" =>
    "Bool", }, "hkbCharacterAddedInfo" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "characterId" => "U64", "instanceName" => "String",
    "templateName" => "String", "fullPathToProject" => "String", "skeleton" => "Pointer",
    "worldFromModel" => "Object|QsTransform", "poseModelSpace" =>
    "Array|Object|QsTransform", }, "hkbCharacterControlCommand" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "characterId" => "U64",
    "command" => "String", "padding" => "I64", }, "hkbCharacterControllerControlData" =>
    phf_ordered_map! { "desiredVelocity" => "Object|Vector4", "verticalGain" => "F64",
    "horizontalCatchUpGain" => "F64", "maxVerticalSeparation" => "F64",
    "maxHorizontalSeparation" => "F64", }, "hkbCharacterControllerModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "controlData" => "Object|hkbCharacterControllerControlData",
    "initialVelocity" => "Object|Vector4", "initialVelocityCoordinates" => "String",
    "motionMode" => "String", "forceDownwardMomentum" => "Bool", "applyGravity" =>
    "Bool", "setInitialVelocity" => "Bool", "isTouchingGround" => "Bool", "gravity" =>
    "Object|Vector4", "timestep" => "F64", "isInitialVelocityAdded" => "Bool", },
    "hkbCharacterControllerModifierInternalState" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "gravity" => "Object|Vector4", "timestep" =>
    "F64", "isInitialVelocityAdded" => "Bool", "isTouchingGround" => "Bool", },
    "hkbCharacterData" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "characterControllerInfo" =>
    "Object|hkbCharacterDataCharacterControllerInfo", "modelUpMS" => "Object|Vector4",
    "modelForwardMS" => "Object|Vector4", "modelRightMS" => "Object|Vector4",
    "characterPropertyInfos" => "Array|Object|hkbVariableInfo", "numBonesPerLod" =>
    "Array|I64", "characterPropertyValues" => "Pointer", "footIkDriverInfo" => "Pointer",
    "handIkDriverInfo" => "Pointer", "stringData" => "Pointer", "mirroredSkeletonInfo" =>
    "Pointer", "scale" => "F64", "numHands" => "I64", "numFloatSlots" => "I64", },
    "hkbCharacterDataCharacterControllerInfo" => phf_ordered_map! { "capsuleHeight" =>
    "F64", "capsuleRadius" => "F64", "collisionFilterInfo" => "U64",
    "characterControllerCinfo" => "Pointer", }, "hkbCharacterInfo" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "characterId" => "U64",
    "event" => "String", "padding" => "I64", }, "hkbCharacterSetup" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "retargetingSkeletonMappers"
    => "Array|Pointer", "animationSkeleton" => "Pointer",
    "ragdollToAnimationSkeletonMapper" => "Pointer", "animationToRagdollSkeletonMapper"
    => "Pointer", "animationBindingSet" => "Pointer", "data" => "Pointer",
    "mirroredSkeleton" => "Pointer", "characterPropertyIdMap" => "Pointer", },
    "hkbCharacterSkinInfo" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "characterId" => "U64", "deformableSkins" => "Array|U64",
    "rigidSkins" => "Array|U64", }, "hkbCharacterSteppedInfo" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "characterId" => "U64",
    "deltaTime" => "F64", "worldFromModel" => "Object|QsTransform", "poseModelSpace" =>
    "Array|Object|QsTransform", "rigidAttachmentTransforms" =>
    "Array|Object|QsTransform", }, "hkbCharacterStringData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "deformableSkinNames" =>
    "Array|String", "rigidSkinNames" => "Array|String", "animationNames" =>
    "Array|String", "animationFilenames" => "Array|String", "characterPropertyNames" =>
    "Array|String", "retargetingSkeletonMapperFilenames" => "Array|String", "lodNames" =>
    "Array|String", "mirroredSyncPointSubstringsA" => "Array|String",
    "mirroredSyncPointSubstringsB" => "Array|String", "name" => "String", "rigName" =>
    "String", "ragdollName" => "String", "behaviorFilename" => "String", },
    "hkbClientCharacterState" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "deformableSkinIds" => "Array|U64", "rigidSkinIds" =>
    "Array|U64", "externalEventIds" => "Array|I64", "auxiliaryInfo" => "Array|Pointer",
    "activeEventIds" => "Array|I64", "activeVariableIds" => "Array|I64", "characterId" =>
    "U64", "instanceName" => "String", "templateName" => "String", "fullPathToProject" =>
    "String", "behaviorData" => "Pointer", "behaviorInternalState" => "Pointer",
    "nodeIdToInternalStateMap" => "Pointer", "visible" => "Bool", "elapsedSimulationTime"
    => "F64", "skeleton" => "Pointer", "worldFromModel" => "Object|QsTransform",
    "poseModelSpace" => "Array|Object|QsTransform", "rigidAttachmentTransforms" =>
    "Array|Object|QsTransform", }, "hkbClipGenerator" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "animationName" => "String", "triggers" => "Pointer",
    "cropStartAmountLocalTime" => "F64", "cropEndAmountLocalTime" => "F64", "startTime"
    => "F64", "playbackSpeed" => "F64", "enforcedDuration" => "F64",
    "userControlledTimeFraction" => "F64", "animationBindingIndex" => "I64", "mode" =>
    "String", "flags" => "I64", "animDatas" => "Array|Null", "animationControl" =>
    "Pointer", "originalTriggers" => "Pointer", "mapperData" => "Pointer", "binding" =>
    "Pointer", "mirroredAnimation" => "Pointer", "extractedMotion" =>
    "Object|QsTransform", "echos" => "Array|Null", "localTime" => "F64", "time" => "F64",
    "previousUserControlledTimeFraction" => "F64", "bufferSize" => "I64",
    "echoBufferSize" => "I64", "atEnd" => "Bool", "ignoreStartTime" => "Bool",
    "pingPongBackward" => "Bool", }, "hkbClipGeneratorEcho" => phf_ordered_map! {
    "offsetLocalTime" => "F64", "weight" => "F64", "dwdt" => "F64", },
    "hkbClipGeneratorInternalState" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "extractedMotion" => "Object|QsTransform", "echos" =>
    "Array|Object|hkbClipGeneratorEcho", "localTime" => "F64", "time" => "F64",
    "previousUserControlledTimeFraction" => "F64", "bufferSize" => "I64",
    "echoBufferSize" => "I64", "atEnd" => "Bool", "ignoreStartTime" => "Bool",
    "pingPongBackward" => "Bool", }, "hkbClipTrigger" => phf_ordered_map! { "localTime"
    => "F64", "event" => "Object|hkbEventProperty", "relativeToEndOfClip" => "Bool",
    "acyclic" => "Bool", "isAnnotation" => "Bool", }, "hkbClipTriggerArray" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "triggers"
    => "Array|Object|hkbClipTrigger", }, "hkbCombineTransformsModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "translationOut" => "Object|Vector4", "rotationOut" =>
    "Object|Quaternion", "leftTranslation" => "Object|Vector4", "leftRotation" =>
    "Object|Quaternion", "rightTranslation" => "Object|Vector4", "rightRotation" =>
    "Object|Quaternion", "invertLeftTransform" => "Bool", "invertRightTransform" =>
    "Bool", "invertResult" => "Bool", }, "hkbCombineTransformsModifierInternalState" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "translationOut" => "Object|Vector4", "rotationOut" => "Object|Quaternion", },
    "hkbCompiledExpressionSet" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "rpn" => "Array|Object|hkbCompiledExpressionSetToken",
    "expressionToRpnIndex" => "Array|I64", "numExpressions" => "I64", },
    "hkbCompiledExpressionSetToken" => phf_ordered_map! { "data" => "F64", "type" =>
    "String", "operator" => "String", }, "hkbComputeDirectionModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "pointIn" => "Object|Vector4", "pointOut" =>
    "Object|Vector4", "groundAngleOut" => "F64", "upAngleOut" => "F64", "verticalOffset"
    => "F64", "reverseGroundAngle" => "Bool", "reverseUpAngle" => "Bool", "projectPoint"
    => "Bool", "normalizePoint" => "Bool", "computeOnlyOnce" => "Bool", "computedOutput"
    => "Bool", }, "hkbComputeDirectionModifierInternalState" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "pointOut" =>
    "Object|Vector4", "groundAngleOut" => "F64", "upAngleOut" => "F64", "computedOutput"
    => "Bool", }, "hkbComputeRotationFromAxisAngleModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "rotationOut" =>
    "Object|Quaternion", "axis" => "Object|Vector4", "angleDegrees" => "F64", },
    "hkbComputeRotationFromAxisAngleModifierInternalState" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "rotationOut" =>
    "Object|Quaternion", }, "hkbComputeRotationToTargetModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "rotationOut" =>
    "Object|Quaternion", "targetPosition" => "Object|Vector4", "currentPosition" =>
    "Object|Vector4", "currentRotation" => "Object|Quaternion", "localAxisOfRotation" =>
    "Object|Vector4", "localFacingDirection" => "Object|Vector4", "resultIsDelta" =>
    "Bool", }, "hkbComputeRotationToTargetModifierInternalState" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "rotationOut" =>
    "Object|Quaternion", }, "hkbCondition" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", }, "hkbContext" => phf_ordered_map! { "character"
    => "Pointer", "behavior" => "Pointer", "nodeToIndexMap" => "Pointer", "eventQueue" =>
    "Pointer", "sharedEventQueue" => "Pointer", "generatorOutputListener" => "Pointer",
    "eventTriggeredTransition" => "Bool", "world" => "Pointer", "attachmentManager" =>
    "Pointer", "animationCache" => "Pointer", }, "hkbDampingModifier" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "kP" => "F64", "kI"
    => "F64", "kD" => "F64", "enableScalarDamping" => "Bool", "enableVectorDamping" =>
    "Bool", "rawValue" => "F64", "dampedValue" => "F64", "rawVector" => "Object|Vector4",
    "dampedVector" => "Object|Vector4", "vecErrorSum" => "Object|Vector4",
    "vecPreviousError" => "Object|Vector4", "errorSum" => "F64", "previousError" =>
    "F64", }, "hkbDampingModifierInternalState" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "dampedVector" => "Object|Vector4",
    "vecErrorSum" => "Object|Vector4", "vecPreviousError" => "Object|Vector4",
    "dampedValue" => "F64", "errorSum" => "F64", "previousError" => "F64", },
    "hkbDefaultMessageLog" => phf_ordered_map! {}, "hkbDelayedModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "modifier" => "Pointer", "delaySeconds" => "F64",
    "durationSeconds" => "F64", "secondsElapsed" => "F64", "isActive" => "Bool", },
    "hkbDelayedModifierInternalState" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "secondsElapsed" => "F64", "isActive" => "Bool", },
    "hkbDetectCloseToGroundModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "closeToGroundEvent" => "Object|hkbEventProperty",
    "closeToGroundHeight" => "F64", "raycastDistanceDown" => "F64", "collisionFilterInfo"
    => "U64", "boneIndex" => "I64", "animBoneIndex" => "I64", "isCloseToGround" =>
    "Bool", }, "hkbDetectCloseToGroundModifierInternalState" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "isCloseToGround" => "Bool",
    }, "hkbEvaluateExpressionModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "expressions" => "Pointer", "compiledExpressionSet"
    => "Pointer", "internalExpressionsData" => "Array|Null", },
    "hkbEvaluateExpressionModifierInternalExpressionData" => phf_ordered_map! {
    "raisedEvent" => "Bool", "wasTrueInPreviousFrame" => "Bool", },
    "hkbEvaluateExpressionModifierInternalState" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "internalExpressionsData" =>
    "Array|Object|hkbEvaluateExpressionModifierInternalExpressionData", },
    "hkbEvaluateHandleModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "handle" => "Pointer", "handlePositionOut" =>
    "Object|Vector4", "handleRotationOut" => "Object|Quaternion", "isValidOut" => "Bool",
    "extrapolationTimeStep" => "F64", "handleChangeSpeed" => "F64", "handleChangeMode" =>
    "String", "oldHandle" => "Object|hkbHandle", "oldHandlePosition" => "Object|Vector4",
    "oldHandleRotation" => "Object|Quaternion", "timeSinceLastModify" => "F64",
    "smoothlyChangingHandles" => "Bool", }, "hkbEvent" => phf_ordered_map! { "id" =>
    "I64", "payload" => "Pointer", "sender" => "Pointer", }, "hkbEventBase" =>
    phf_ordered_map! { "id" => "I64", "payload" => "Pointer", }, "hkbEventDrivenModifier"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "modifier" => "Pointer", "activateEventId" => "I64",
    "deactivateEventId" => "I64", "activeByDefault" => "Bool", "isActive" => "Bool", },
    "hkbEventDrivenModifierInternalState" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "isActive" => "Bool", }, "hkbEventInfo" =>
    phf_ordered_map! { "flags" => "String", }, "hkbEventPayload" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", }, "hkbEventPayloadList" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "payloads"
    => "Array|Pointer", }, "hkbEventProperty" => phf_ordered_map! { "id" => "I64",
    "payload" => "Pointer", }, "hkbEventRaisedInfo" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "characterId" => "U64",
    "eventName" => "String", "raisedBySdk" => "Bool", "senderId" => "I64", "padding" =>
    "I64", }, "hkbEventRangeData" => phf_ordered_map! { "upperBound" => "F64", "event" =>
    "Object|hkbEventProperty", "eventMode" => "String", }, "hkbEventRangeDataArray" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "eventData"
    => "Array|Object|hkbEventRangeData", }, "hkbEventSequencedData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "events" =>
    "Array|Object|hkbEventSequencedDataSequencedEvent", },
    "hkbEventSequencedDataSequencedEvent" => phf_ordered_map! { "event" =>
    "Object|hkbEvent", "time" => "F64", }, "hkbEventsFromRangeModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "inputValue" => "F64", "lowerBound" => "F64", "eventRanges"
    => "Pointer", "wasActiveInPreviousFrame" => "Array|Null", },
    "hkbEventsFromRangeModifierInternalState" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "wasActiveInPreviousFrame" => "Array|Bool", },
    "hkbExpressionCondition" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "expression" => "String", "compiledExpressionSet" =>
    "Pointer", }, "hkbExpressionData" => phf_ordered_map! { "expression" => "String",
    "assignmentVariableIndex" => "I64", "assignmentEventIndex" => "I64", "eventMode" =>
    "String", "raisedEvent" => "Bool", "wasTrueInPreviousFrame" => "Bool", },
    "hkbExpressionDataArray" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "expressionsData" => "Array|Object|hkbExpressionData", },
    "hkbExtractRagdollPoseModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "poseMatchingBone0" => "I64", "poseMatchingBone1" =>
    "I64", "poseMatchingBone2" => "I64", "enableComputeWorldFromModel" => "Bool", },
    "hkbFootIkControlData" => phf_ordered_map! { "gains" => "Object|hkbFootIkGains", },
    "hkbFootIkControlsModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "controlData" => "Object|hkbFootIkControlData",
    "legs" => "Array|Object|hkbFootIkControlsModifierLeg", "errorOutTranslation" =>
    "Object|Vector4", "alignWithGroundRotation" => "Object|Quaternion", },
    "hkbFootIkControlsModifierLeg" => phf_ordered_map! { "groundPosition" =>
    "Object|Vector4", "ungroundedEvent" => "Object|hkbEventProperty", "verticalError" =>
    "F64", "hitSomething" => "Bool", "isPlantedMS" => "Bool", }, "hkbFootIkDriverInfo" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "legs" =>
    "Array|Object|hkbFootIkDriverInfoLeg", "raycastDistanceUp" => "F64",
    "raycastDistanceDown" => "F64", "originalGroundHeightMS" => "F64", "verticalOffset"
    => "F64", "collisionFilterInfo" => "U64", "forwardAlignFraction" => "F64",
    "sidewaysAlignFraction" => "F64", "sidewaysSampleWidth" => "F64",
    "lockFeetWhenPlanted" => "Bool", "useCharacterUpVector" => "Bool",
    "isQuadrupedNarrow" => "Bool", }, "hkbFootIkDriverInfoLeg" => phf_ordered_map! {
    "prevAnkleRotLS" => "Object|Quaternion", "kneeAxisLS" => "Object|Vector4",
    "footEndLS" => "Object|Vector4", "footPlantedAnkleHeightMS" => "F64",
    "footRaisedAnkleHeightMS" => "F64", "maxAnkleHeightMS" => "F64", "minAnkleHeightMS"
    => "F64", "maxKneeAngleDegrees" => "F64", "minKneeAngleDegrees" => "F64",
    "maxAnkleAngleDegrees" => "F64", "hipIndex" => "I64", "kneeIndex" => "I64",
    "ankleIndex" => "I64", }, "hkbFootIkGains" => phf_ordered_map! { "onOffGain" =>
    "F64", "groundAscendingGain" => "F64", "groundDescendingGain" => "F64",
    "footPlantedGain" => "F64", "footRaisedGain" => "F64", "footUnlockGain" => "F64",
    "worldFromModelFeedbackGain" => "F64", "errorUpDownBias" => "F64",
    "alignWorldFromModelGain" => "F64", "hipOrientationGain" => "F64",
    "maxKneeAngleDifference" => "F64", "ankleOrientationGain" => "F64", },
    "hkbFootIkModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "gains" => "Object|hkbFootIkGains", "legs" =>
    "Array|Object|hkbFootIkModifierLeg", "raycastDistanceUp" => "F64",
    "raycastDistanceDown" => "F64", "originalGroundHeightMS" => "F64", "errorOut" =>
    "F64", "errorOutTranslation" => "Object|Vector4", "alignWithGroundRotation" =>
    "Object|Quaternion", "verticalOffset" => "F64", "collisionFilterInfo" => "U64",
    "forwardAlignFraction" => "F64", "sidewaysAlignFraction" => "F64",
    "sidewaysSampleWidth" => "F64", "useTrackData" => "Bool", "lockFeetWhenPlanted" =>
    "Bool", "useCharacterUpVector" => "Bool", "alignMode" => "String", "internalLegData"
    => "Array|Object|hkbFootIkModifierInternalLegData", "prevIsFootIkEnabled" => "F64",
    "isSetUp" => "Bool", "isGroundPositionValid" => "Bool", "timeStep" => "F64", },
    "hkbFootIkModifierInternalLegData" => phf_ordered_map! { "groundPosition" =>
    "Object|Vector4", "footIkSolver" => "Pointer", }, "hkbFootIkModifierLeg" =>
    phf_ordered_map! { "originalAnkleTransformMS" => "Object|QsTransform",
    "prevAnkleRotLS" => "Object|Quaternion", "kneeAxisLS" => "Object|Vector4",
    "footEndLS" => "Object|Vector4", "ungroundedEvent" => "Object|hkbEventProperty",
    "footPlantedAnkleHeightMS" => "F64", "footRaisedAnkleHeightMS" => "F64",
    "maxAnkleHeightMS" => "F64", "minAnkleHeightMS" => "F64", "maxKneeAngleDegrees" =>
    "F64", "minKneeAngleDegrees" => "F64", "verticalError" => "F64",
    "maxAnkleAngleDegrees" => "F64", "hipIndex" => "I64", "kneeIndex" => "I64",
    "ankleIndex" => "I64", "hitSomething" => "Bool", "isPlantedMS" => "Bool",
    "isOriginalAnkleTransformMSSet" => "Bool", }, "hkbGenerator" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", }, "hkbGeneratorOutputListener" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", }, "hkbGeneratorSyncInfo" =>
    phf_ordered_map! { "syncPoints" => "Object|hkbGeneratorSyncInfoSyncPoint",
    "baseFrequency" => "F64", "localTime" => "F64", "playbackSpeed" => "F64",
    "numSyncPoints" => "I64", "isCyclic" => "Bool", "isMirrored" => "Bool", "isAdditive"
    => "Bool", }, "hkbGeneratorSyncInfoSyncPoint" => phf_ordered_map! { "id" => "I64",
    "time" => "F64", }, "hkbGeneratorTransitionEffect" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "selfTransitionMode" => "String", "eventMode" => "String",
    "defaultEventMode" => "String", "transitionGenerator" => "Pointer", "blendInDuration"
    => "F64", "blendOutDuration" => "F64", "syncToGeneratorStartTime" => "Bool",
    "fromGenerator" => "Pointer", "toGenerator" => "Pointer", "timeInTransition" =>
    "F64", "duration" => "F64", "effectiveBlendInDuration" => "F64",
    "effectiveBlendOutDuration" => "F64", "toGeneratorState" => "String",
    "echoTransitionGenerator" => "Bool", "echoToGenerator" => "Bool", "justActivated" =>
    "Bool", "updateActiveNodes" => "Bool", "stage" => "String", },
    "hkbGeneratorTransitionEffectInternalState" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "timeInTransition" => "F64", "duration" =>
    "F64", "effectiveBlendInDuration" => "F64", "effectiveBlendOutDuration" => "F64",
    "toGeneratorState" => "String", "echoTransitionGenerator" => "Bool",
    "echoToGenerator" => "Bool", "justActivated" => "Bool", "updateActiveNodes" =>
    "Bool", "stage" => "String", }, "hkbGetHandleOnBoneModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "handleOut" =>
    "Pointer", "localFrameName" => "String", "ragdollBoneIndex" => "I64",
    "animationBoneIndex" => "I64", }, "hkbGetUpModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "groundNormal" =>
    "Object|Vector4", "duration" => "F64", "alignWithGroundDuration" => "F64",
    "rootBoneIndex" => "I64", "otherBoneIndex" => "I64", "anotherBoneIndex" => "I64",
    "timeSinceBegin" => "F64", "timeStep" => "F64", "initNextModify" => "Bool", },
    "hkbGetUpModifierInternalState" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "timeSinceBegin" => "F64", "timeStep" => "F64",
    "initNextModify" => "Bool", }, "hkbGetWorldFromModelModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "translationOut" =>
    "Object|Vector4", "rotationOut" => "Object|Quaternion", },
    "hkbGetWorldFromModelModifierInternalState" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "translationOut" => "Object|Vector4",
    "rotationOut" => "Object|Quaternion", }, "hkbHandIkControlData" => phf_ordered_map! {
    "targetPosition" => "Object|Vector4", "targetRotation" => "Object|Quaternion",
    "targetNormal" => "Object|Vector4", "targetHandle" => "Pointer",
    "transformOnFraction" => "F64", "normalOnFraction" => "F64", "fadeInDuration" =>
    "F64", "fadeOutDuration" => "F64", "extrapolationTimeStep" => "F64",
    "handleChangeSpeed" => "F64", "handleChangeMode" => "String", "fixUp" => "Bool", },
    "hkbHandIkControlsModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "hands" =>
    "Array|Object|hkbHandIkControlsModifierHand", }, "hkbHandIkControlsModifierHand" =>
    phf_ordered_map! { "controlData" => "Object|hkbHandIkControlData", "handIndex" =>
    "I64", "enable" => "Bool", }, "hkbHandIkDriverInfo" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "hands" =>
    "Array|Object|hkbHandIkDriverInfoHand", "fadeInOutCurve" => "String", },
    "hkbHandIkDriverInfoHand" => phf_ordered_map! { "elbowAxisLS" => "Object|Vector4",
    "backHandNormalLS" => "Object|Vector4", "handOffsetLS" => "Object|Vector4",
    "handOrienationOffsetLS" => "Object|Quaternion", "maxElbowAngleDegrees" => "F64",
    "minElbowAngleDegrees" => "F64", "shoulderIndex" => "I64", "shoulderSiblingIndex" =>
    "I64", "elbowIndex" => "I64", "elbowSiblingIndex" => "I64", "wristIndex" => "I64",
    "enforceEndPosition" => "Bool", "enforceEndRotation" => "Bool", "localFrameName" =>
    "String", }, "hkbHandIkModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "hands" => "Array|Object|hkbHandIkModifierHand",
    "fadeInOutCurve" => "String", "internalHandData" => "Array|Null", },
    "hkbHandIkModifierHand" => phf_ordered_map! { "elbowAxisLS" => "Object|Vector4",
    "backHandNormalLS" => "Object|Vector4", "handOffsetLS" => "Object|Vector4",
    "handOrienationOffsetLS" => "Object|Quaternion", "maxElbowAngleDegrees" => "F64",
    "minElbowAngleDegrees" => "F64", "shoulderIndex" => "I64", "shoulderSiblingIndex" =>
    "I64", "elbowIndex" => "I64", "elbowSiblingIndex" => "I64", "wristIndex" => "I64",
    "enforceEndPosition" => "Bool", "enforceEndRotation" => "Bool", "localFrameName" =>
    "String", }, "hkbHandle" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "frame" => "Pointer", "rigidBody" => "Pointer",
    "character" => "Pointer", "animationBoneIndex" => "I64", }, "hkbIntEventPayload" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "data" =>
    "I64", }, "hkbIntVariableSequencedData" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "samples" =>
    "Array|Object|hkbIntVariableSequencedDataSample", "variableIndex" => "I64", },
    "hkbIntVariableSequencedDataSample" => phf_ordered_map! { "time" => "F64", "value" =>
    "I64", }, "hkbKeyframeBonesModifier" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "variableBindingSet" => "Pointer",
    "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool", "userData" =>
    "U64", "name" => "String", "id" => "I64", "cloneState" => "String", "padNode" =>
    "Bool", "enable" => "Bool", "padModifier" => "Bool", "keyframeInfo" =>
    "Array|Object|hkbKeyframeBonesModifierKeyframeInfo", "keyframedBonesList" =>
    "Pointer", }, "hkbKeyframeBonesModifierKeyframeInfo" => phf_ordered_map! {
    "keyframedPosition" => "Object|Vector4", "keyframedRotation" => "Object|Quaternion",
    "boneIndex" => "I64", "isValid" => "Bool", }, "hkbLinkedSymbolInfo" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "eventNames" => "Array|String", "variableNames" => "Array|String", },
    "hkbLookAtModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "targetWS" => "Object|Vector4", "headForwardLS" =>
    "Object|Vector4", "neckForwardLS" => "Object|Vector4", "neckRightLS" =>
    "Object|Vector4", "eyePositionHS" => "Object|Vector4", "newTargetGain" => "F64",
    "onGain" => "F64", "offGain" => "F64", "limitAngleDegrees" => "F64", "limitAngleLeft"
    => "F64", "limitAngleRight" => "F64", "limitAngleUp" => "F64", "limitAngleDown" =>
    "F64", "headIndex" => "I64", "neckIndex" => "I64", "isOn" => "Bool",
    "individualLimitsOn" => "Bool", "isTargetInsideLimitCone" => "Bool",
    "lookAtLastTargetWS" => "Object|Vector4", "lookAtWeight" => "F64", },
    "hkbLookAtModifierInternalState" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "lookAtLastTargetWS" => "Object|Vector4", "lookAtWeight"
    => "F64", "isTargetInsideLimitCone" => "Bool", }, "hkbManualSelectorGenerator" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "generators" =>
    "Array|Pointer", "selectedGeneratorIndex" => "I64", "currentGeneratorIndex" => "I64",
    }, "hkbManualSelectorGeneratorInternalState" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "currentGeneratorIndex" => "I64", },
    "hkbMessageLog" => phf_ordered_map! { "messages" => "Pointer", "maxMessages" =>
    "I64", }, "hkbMirrorModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "isAdditive" => "Bool", }, "hkbMirroredSkeletonInfo"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "mirrorAxis" => "Object|Vector4", "bonePairMap" => "Array|I64", }, "hkbModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", }, "hkbModifierGenerator" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "modifier" => "Pointer", "generator" => "Pointer", },
    "hkbModifierList" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "modifiers" => "Array|Pointer", }, "hkbModifierWrapper" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "modifier" => "Pointer", }, "hkbMoveCharacterModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "offsetPerSecondMS" => "Object|Vector4",
    "timeSinceLastModify" => "F64", }, "hkbMoveCharacterModifierInternalState" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "timeSinceLastModify" => "F64", }, "hkbNamedEventPayload" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "name" => "String", },
    "hkbNamedIntEventPayload" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "name" => "String", "data" => "I64", },
    "hkbNamedRealEventPayload" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "name" => "String", "data" => "F64", },
    "hkbNamedStringEventPayload" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "name" => "String", "data" => "String", }, "hkbNode" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", }, "hkbNodeInternalStateInfo"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "syncInfo" => "Object|hkbGeneratorSyncInfo", "name" => "String", "internalState" =>
    "Pointer", "nodeId" => "I64", "hasActivateBeenCalled" => "Bool", },
    "hkbParticleSystemEventPayload" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "type" => "String", "emitBoneIndex" => "I64", "offset" =>
    "Object|Vector4", "direction" => "Object|Vector4", "numParticles" => "I64", "speed"
    => "F64", }, "hkbPoseMatchingGenerator" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "variableBindingSet" => "Pointer",
    "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool", "userData" =>
    "U64", "name" => "String", "id" => "I64", "cloneState" => "String", "padNode" =>
    "Bool", "referencePoseWeightThreshold" => "F64", "blendParameter" => "F64",
    "minCyclicBlendParameter" => "F64", "maxCyclicBlendParameter" => "F64",
    "indexOfSyncMasterChild" => "I64", "flags" => "I64", "subtractLastChild" => "Bool",
    "children" => "Array|Pointer", "childrenInternalStates" => "Array|Null",
    "sortedChildren" => "Array|Null", "endIntervalWeight" => "F64", "numActiveChildren"
    => "I64", "beginIntervalIndex" => "I64", "endIntervalIndex" => "I64", "initSync" =>
    "Bool", "doSubtractiveBlend" => "Bool", "worldFromModelRotation" =>
    "Object|Quaternion", "blendSpeed" => "F64", "minSpeedToSwitch" => "F64",
    "minSwitchTimeNoError" => "F64", "minSwitchTimeFullError" => "F64",
    "startPlayingEventId" => "I64", "startMatchingEventId" => "I64", "rootBoneIndex" =>
    "I64", "otherBoneIndex" => "I64", "anotherBoneIndex" => "I64", "pelvisIndex" =>
    "I64", "mode" => "String", "currentMatch" => "I64", "bestMatch" => "I64",
    "timeSinceBetterMatch" => "F64", "error" => "F64", "resetCurrentMatchLocalTime" =>
    "Bool", "poseMatchingUtility" => "Pointer", },
    "hkbPoseMatchingGeneratorInternalState" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "currentMatch" => "I64", "bestMatch" => "I64",
    "timeSinceBetterMatch" => "F64", "error" => "F64", "resetCurrentMatchLocalTime" =>
    "Bool", }, "hkbPoweredRagdollControlData" => phf_ordered_map! { "maxForce" => "F64",
    "tau" => "F64", "damping" => "F64", "proportionalRecoveryVelocity" => "F64",
    "constantRecoveryVelocity" => "F64", }, "hkbPoweredRagdollControlsModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "controlData" => "Object|hkbPoweredRagdollControlData",
    "bones" => "Pointer", "worldFromModelModeData" => "Object|hkbWorldFromModelModeData",
    "boneWeights" => "Pointer", }, "hkbProjectData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "worldUpWS" =>
    "Object|Vector4", "stringData" => "Pointer", "defaultEventMode" => "String", },
    "hkbProjectStringData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "animationFilenames" => "Array|String",
    "behaviorFilenames" => "Array|String", "characterFilenames" => "Array|String",
    "eventNames" => "Array|String", "animationPath" => "String", "behaviorPath" =>
    "String", "characterPath" => "String", "fullPathToSource" => "String", "rootPath" =>
    "String", }, "hkbProxyModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "proxyInfo" => "Object|hkbProxyModifierProxyInfo",
    "linearVelocity" => "Object|Vector4", "horizontalGain" => "F64", "verticalGain" =>
    "F64", "maxHorizontalSeparation" => "F64", "maxVerticalSeparation" => "F64",
    "verticalDisplacementError" => "F64", "verticalDisplacementErrorGain" => "F64",
    "maxVerticalDisplacement" => "F64", "minVerticalDisplacement" => "F64",
    "capsuleHeight" => "F64", "capsuleRadius" => "F64", "maxSlopeForRotation" => "F64",
    "collisionFilterInfo" => "U64", "phantomType" => "String", "linearVelocityMode" =>
    "String", "ignoreIncomingRotation" => "Bool", "ignoreCollisionDuringRotation" =>
    "Bool", "ignoreIncomingTranslation" => "Bool", "includeDownwardMomentum" => "Bool",
    "followWorldFromModel" => "Bool", "isTouchingGround" => "Bool", "characterProxy" =>
    "Pointer", "phantom" => "Pointer", "phantomShape" => "Pointer",
    "horizontalDisplacement" => "Object|Vector4", "verticalDisplacement" => "F64",
    "timestep" => "F64", "previousFrameFollowWorldFromModel" => "Bool", },
    "hkbProxyModifierProxyInfo" => phf_ordered_map! { "dynamicFriction" => "F64",
    "staticFriction" => "F64", "keepContactTolerance" => "F64", "up" => "Object|Vector4",
    "keepDistance" => "F64", "contactAngleSensitivity" => "F64", "userPlanes" => "U64",
    "maxCharacterSpeedForSolver" => "F64", "characterStrength" => "F64", "characterMass"
    => "F64", "maxSlope" => "F64", "penetrationRecoverySpeed" => "F64",
    "maxCastIterations" => "I64", "refreshManifoldInCheckSupport" => "Bool", },
    "hkbRaiseEventCommand" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "characterId" => "U64", "global" => "Bool", "externalId"
    => "I64", }, "hkbRealEventPayload" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "data" => "F64", }, "hkbRealVariableSequencedData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "samples"
    => "Array|Object|hkbRealVariableSequencedDataSample", "variableIndex" => "I64", },
    "hkbRealVariableSequencedDataSample" => phf_ordered_map! { "time" => "F64", "value"
    => "F64", }, "hkbReferencePoseGenerator" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "variableBindingSet" => "Pointer",
    "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool", "userData" =>
    "U64", "name" => "String", "id" => "I64", "cloneState" => "String", "padNode" =>
    "Bool", "skeleton" => "Pointer", }, "hkbRegisteredGenerator" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "generator" => "Pointer", "relativePosition" => "Object|Vector4", "relativeDirection"
    => "Object|Vector4", }, "hkbRigidBodyRagdollControlData" => phf_ordered_map! {
    "keyFrameHierarchyControlData" => "Object|hkaKeyFrameHierarchyUtilityControlData",
    "durationToBlend" => "F64", }, "hkbRigidBodyRagdollControlsModifier" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "controlData" => "Object|hkbRigidBodyRagdollControlData",
    "bones" => "Pointer", }, "hkbRoleAttribute" => phf_ordered_map! { "role" => "String",
    "flags" => "String", }, "hkbRotateCharacterModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "variableBindingSet" =>
    "Pointer", "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool",
    "userData" => "U64", "name" => "String", "id" => "I64", "cloneState" => "String",
    "padNode" => "Bool", "enable" => "Bool", "padModifier" => "Bool", "degreesPerSecond"
    => "F64", "speedMultiplier" => "F64", "axisOfRotation" => "Object|Vector4", "angle"
    => "F64", }, "hkbRotateCharacterModifierInternalState" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "angle" => "F64", },
    "hkbSenseHandleModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "handle" => "Object|hkbHandle", "sensorLocalOffset"
    => "Object|Vector4", "ranges" => "Array|Object|hkbSenseHandleModifierRange",
    "handleOut" => "Pointer", "handleIn" => "Pointer", "localFrameName" => "String",
    "sensorLocalFrameName" => "String", "minDistance" => "F64", "maxDistance" => "F64",
    "distanceOut" => "F64", "collisionFilterInfo" => "U64", "sensorRagdollBoneIndex" =>
    "I64", "sensorAnimationBoneIndex" => "I64", "sensingMode" => "String",
    "extrapolateSensorPosition" => "Bool", "keepFirstSensedHandle" => "Bool",
    "foundHandleOut" => "Bool", "timeSinceLastModify" => "F64",
    "rangeIndexForEventToSendNextUpdate" => "I64", }, "hkbSenseHandleModifierRange" =>
    phf_ordered_map! { "event" => "Object|hkbEventProperty", "minDistance" => "F64",
    "maxDistance" => "F64", "ignoreHandle" => "Bool", }, "hkbSequence" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "eventSequencedData" => "Array|Pointer",
    "realVariableSequencedData" => "Array|Pointer", "boolVariableSequencedData" =>
    "Array|Pointer", "intVariableSequencedData" => "Array|Pointer", "enableEventId" =>
    "I64", "disableEventId" => "I64", "stringData" => "Pointer", "variableIdMap" =>
    "Pointer", "eventIdMap" => "Pointer", "nextSampleEvents" => "Array|Null",
    "nextSampleReals" => "Array|Null", "nextSampleBools" => "Array|Null",
    "nextSampleInts" => "Array|Null", "time" => "F64", "isEnabled" => "Bool", },
    "hkbSequenceInternalState" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "nextSampleEvents" => "Array|I64", "nextSampleReals" =>
    "Array|I64", "nextSampleBools" => "Array|I64", "nextSampleInts" => "Array|I64",
    "time" => "F64", "isEnabled" => "Bool", }, "hkbSequenceStringData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "eventNames" => "Array|String", "variableNames" => "Array|String", },
    "hkbSequencedData" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", }, "hkbSetBehaviorCommand" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "characterId" => "U64", "behavior" => "Pointer",
    "rootGenerator" => "Pointer", "referencedBehaviors" => "Array|Pointer",
    "startStateIndex" => "I64", "randomizeSimulation" => "Bool", "padding" => "I64", },
    "hkbSetLocalTimeOfClipGeneratorCommand" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "characterId" => "U64", "localTime" => "F64",
    "nodeId" => "I64", }, "hkbSetNodePropertyCommand" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "characterId" => "U64",
    "nodeName" => "String", "propertyName" => "String", "propertyValue" =>
    "Object|hkbVariableValue", "padding" => "I64", }, "hkbSetWordVariableCommand" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "quadValue"
    => "Object|Vector4", "characterId" => "U64", "variableId" => "I64", "value" =>
    "Object|hkbVariableValue", "type" => "String", "global" => "Bool", },
    "hkbSetWorldFromModelModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "translation" => "Object|Vector4", "rotation" =>
    "Object|Quaternion", "setTranslation" => "Bool", "setRotation" => "Bool", },
    "hkbSimulationControlCommand" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "command" => "String", }, "hkbSimulationStateInfo" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "simulationState" => "String", }, "hkbStateChooser" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", }, "hkbStateListener" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", },
    "hkbStateMachine" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool",
    "eventToSendWhenStateOrTransitionChanges" => "Object|hkbEvent", "startStateChooser"
    => "Pointer", "startStateId" => "I64", "returnToPreviousStateEventId" => "I64",
    "randomTransitionEventId" => "I64", "transitionToNextHigherStateEventId" => "I64",
    "transitionToNextLowerStateEventId" => "I64", "syncVariableIndex" => "I64",
    "currentStateId" => "I64", "wrapAroundStateId" => "Bool",
    "maxSimultaneousTransitions" => "I64", "startStateMode" => "String",
    "selfTransitionMode" => "String", "isActive" => "Bool", "states" => "Array|Pointer",
    "wildcardTransitions" => "Pointer", "stateIdToIndexMap" => "Pointer",
    "activeTransitions" => "Array|Null", "transitionFlags" => "Array|Null",
    "wildcardTransitionFlags" => "Array|Null", "delayedTransitions" => "Array|Null",
    "timeInState" => "F64", "lastLocalTime" => "F64", "previousStateId" => "I64",
    "nextStartStateIndexOverride" => "I64", "stateOrTransitionChanged" => "Bool",
    "echoNextUpdate" => "Bool", "sCurrentStateIndexAndEntered" => "U64", },
    "hkbStateMachineActiveTransitionInfo" => phf_ordered_map! { "transitionEffect" =>
    "Pointer", "transitionEffectInternalStateInfo" => "Pointer",
    "transitionInfoReference" => "Object|hkbStateMachineTransitionInfoReference",
    "transitionInfoReferenceForTE" => "Object|hkbStateMachineTransitionInfoReference",
    "fromStateId" => "I64", "toStateId" => "I64", "isReturnToPreviousState" => "Bool", },
    "hkbStateMachineDelayedTransitionInfo" => phf_ordered_map! { "delayedTransition" =>
    "Object|hkbStateMachineProspectiveTransitionInfo", "timeDelayed" => "F64",
    "isDelayedTransitionReturnToPreviousState" => "Bool", "wasInAbutRangeLastFrame" =>
    "Bool", }, "hkbStateMachineEventPropertyArray" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "events" =>
    "Array|Object|hkbEventProperty", }, "hkbStateMachineInternalState" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "activeTransitions" => "Array|Object|hkbStateMachineActiveTransitionInfo",
    "transitionFlags" => "Array|U64", "wildcardTransitionFlags" => "Array|U64",
    "delayedTransitions" => "Array|Object|hkbStateMachineDelayedTransitionInfo",
    "timeInState" => "F64", "lastLocalTime" => "F64", "currentStateId" => "I64",
    "previousStateId" => "I64", "nextStartStateIndexOverride" => "I64",
    "stateOrTransitionChanged" => "Bool", "echoNextUpdate" => "Bool", },
    "hkbStateMachineNestedStateMachineData" => phf_ordered_map! { "nestedStateMachine" =>
    "Pointer", "eventIdMap" => "Pointer", }, "hkbStateMachineProspectiveTransitionInfo"
    => phf_ordered_map! { "transitionInfoReference" =>
    "Object|hkbStateMachineTransitionInfoReference", "transitionInfoReferenceForTE" =>
    "Object|hkbStateMachineTransitionInfoReference", "toStateId" => "I64", },
    "hkbStateMachineStateInfo" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "listeners" => "Array|Pointer",
    "enterNotifyEvents" => "Pointer", "exitNotifyEvents" => "Pointer", "transitions" =>
    "Pointer", "generator" => "Pointer", "name" => "String", "stateId" => "I64",
    "probability" => "F64", "enable" => "Bool", }, "hkbStateMachineTimeInterval" =>
    phf_ordered_map! { "enterEventId" => "I64", "exitEventId" => "I64", "enterTime" =>
    "F64", "exitTime" => "F64", }, "hkbStateMachineTransitionInfo" => phf_ordered_map! {
    "triggerInterval" => "Object|hkbStateMachineTimeInterval", "initiateInterval" =>
    "Object|hkbStateMachineTimeInterval", "transition" => "Pointer", "condition" =>
    "Pointer", "eventId" => "I64", "toStateId" => "I64", "fromNestedStateId" => "I64",
    "toNestedStateId" => "I64", "priority" => "I64", "flags" => "String", },
    "hkbStateMachineTransitionInfoArray" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "transitions" =>
    "Array|Object|hkbStateMachineTransitionInfo", },
    "hkbStateMachineTransitionInfoReference" => phf_ordered_map! { "fromStateIndex" =>
    "I64", "transitionIndex" => "I64", "stateMachineId" => "I64", }, "hkbStringCondition"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "conditionString" => "String", }, "hkbStringEventPayload" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "data" => "String", },
    "hkbTestStateChooser" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "int" => "I64", "real" => "F64", "string" => "String", },
    "hkbTimerModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "alarmTimeSeconds" => "F64", "alarmEvent" =>
    "Object|hkbEventProperty", "secondsElapsed" => "F64", },
    "hkbTimerModifierInternalState" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "secondsElapsed" => "F64", }, "hkbTransformVectorModifier"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "variableBindingSet" => "Pointer", "cachedBindables" => "Array|Null",
    "areBindablesCached" => "Bool", "userData" => "U64", "name" => "String", "id" =>
    "I64", "cloneState" => "String", "padNode" => "Bool", "enable" => "Bool",
    "padModifier" => "Bool", "rotation" => "Object|Quaternion", "translation" =>
    "Object|Vector4", "vectorIn" => "Object|Vector4", "vectorOut" => "Object|Vector4",
    "rotateOnly" => "Bool", "inverse" => "Bool", "computeOnActivate" => "Bool",
    "computeOnModify" => "Bool", }, "hkbTransformVectorModifierInternalState" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "vectorOut"
    => "Object|Vector4", }, "hkbTransitionEffect" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "variableBindingSet" => "Pointer",
    "cachedBindables" => "Array|Null", "areBindablesCached" => "Bool", "userData" =>
    "U64", "name" => "String", "id" => "I64", "cloneState" => "String", "padNode" =>
    "Bool", "selfTransitionMode" => "String", "eventMode" => "String", "defaultEventMode"
    => "String", }, "hkbTwistModifier" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "variableBindingSet" => "Pointer", "cachedBindables" =>
    "Array|Null", "areBindablesCached" => "Bool", "userData" => "U64", "name" =>
    "String", "id" => "I64", "cloneState" => "String", "padNode" => "Bool", "enable" =>
    "Bool", "padModifier" => "Bool", "axisOfRotation" => "Object|Vector4", "twistAngle"
    => "F64", "startBoneIndex" => "I64", "endBoneIndex" => "I64", "setAngleMethod" =>
    "String", "rotationAxisCoordinates" => "String", "isAdditive" => "Bool",
    "boneChainIndices" => "Array|Null", "parentBoneIndices" => "Array|Null", },
    "hkbVariableBindingSet" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "bindings" => "Array|Object|hkbVariableBindingSetBinding",
    "indexOfBindingToEnable" => "I64", "hasOutputBinding" => "Bool", },
    "hkbVariableBindingSetBinding" => phf_ordered_map! { "memberPath" => "String",
    "memberClass" => "Pointer", "offsetInObjectPlusOne" => "I64", "offsetInArrayPlusOne"
    => "I64", "rootVariableIndex" => "I64", "variableIndex" => "I64", "bitIndex" =>
    "I64", "bindingType" => "String", "memberType" => "String", "variableType" => "I64",
    "flags" => "String", }, "hkbVariableInfo" => phf_ordered_map! { "role" =>
    "Object|hkbRoleAttribute", "type" => "String", }, "hkbVariableValue" =>
    phf_ordered_map! { "value" => "I64", }, "hkbVariableValueSet" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "wordVariableValues" =>
    "Array|Object|hkbVariableValue", "quadVariableValues" => "Array|Object|Vector4",
    "variantVariableValues" => "Array|Pointer", }, "hkbWorldEnums" => phf_ordered_map!
    {}, "hkbWorldFromModelModeData" => phf_ordered_map! { "poseMatchingBone0" => "I64",
    "poseMatchingBone1" => "I64", "poseMatchingBone2" => "I64", "mode" => "String", },
    "hkp2dAngConstraintAtom" => phf_ordered_map! { "type" => "String", "freeRotationAxis"
    => "U64", }, "hkpAabbPhantom" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "world" => "Pointer", "userData" => "U64", "collidable" =>
    "Object|hkpLinkedCollidable", "multiThreadCheck" => "Object|hkMultiThreadCheck",
    "name" => "String", "properties" => "Array|Object|hkpProperty", "treeData" =>
    "Pointer", "overlapListeners" => "Array|Pointer", "phantomListeners" =>
    "Array|Pointer", "aabb" => "Object|hkAabb", "overlappingCollidables" =>
    "Array|Pointer", "orderDirty" => "Bool", }, "hkpAction" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" => "Pointer", "island"
    => "Pointer", "userData" => "U64", "name" => "String", }, "hkpAgent1nSector" =>
    phf_ordered_map! { "bytesAllocated" => "U64", "pad0" => "U64", "pad1" => "U64",
    "pad2" => "U64", "data" => "U64", }, "hkpAngConstraintAtom" => phf_ordered_map! {
    "type" => "String", "firstConstrainedAxis" => "U64", "numConstrainedAxes" => "U64",
    }, "hkpAngFrictionConstraintAtom" => phf_ordered_map! { "type" => "String",
    "isEnabled" => "U64", "firstFrictionAxis" => "U64", "numFrictionAxes" => "U64",
    "maxFrictionTorque" => "F64", }, "hkpAngLimitConstraintAtom" => phf_ordered_map! {
    "type" => "String", "isEnabled" => "U64", "limitAxis" => "U64", "minAngle" => "F64",
    "maxAngle" => "F64", "angularLimitsTauFactor" => "F64", },
    "hkpAngMotorConstraintAtom" => phf_ordered_map! { "type" => "String", "isEnabled" =>
    "Bool", "motorAxis" => "U64", "initializedOffset" => "I64",
    "previousTargetAngleOffset" => "I64", "correspondingAngLimitSolverResultOffset" =>
    "I64", "targetAngle" => "F64", "motor" => "Pointer", }, "hkpAngularDashpotAction" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" =>
    "Pointer", "island" => "Pointer", "userData" => "U64", "name" => "String", "entityA"
    => "Pointer", "entityB" => "Pointer", "rotation" => "Object|Quaternion", "strength"
    => "F64", "damping" => "F64", }, "hkpArrayAction" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" => "Pointer", "island"
    => "Pointer", "userData" => "U64", "name" => "String", "entities" => "Array|Pointer",
    }, "hkpBallAndSocketConstraintData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "atoms" =>
    "Object|hkpBallAndSocketConstraintDataAtoms", },
    "hkpBallAndSocketConstraintDataAtoms" => phf_ordered_map! { "pivots" =>
    "Object|hkpSetLocalTranslationsConstraintAtom", "setupStabilization" =>
    "Object|hkpSetupStabilizationAtom", "ballSocket" =>
    "Object|hkpBallSocketConstraintAtom", }, "hkpBallGun" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" => "String", "name" =>
    "String", "keyboardKey" => "String", "listeners" => "Array|Pointer", "bulletRadius"
    => "F64", "bulletVelocity" => "F64", "bulletMass" => "F64", "damageMultiplier" =>
    "F64", "maxBulletsInWorld" => "I64", "bulletOffsetFromCenter" => "Object|Vector4",
    "addedBodies" => "Pointer", }, "hkpBallSocketChainData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "atoms"
    => "Object|hkpBridgeAtoms", "infos" =>
    "Array|Object|hkpBallSocketChainDataConstraintInfo", "tau" => "F64", "damping" =>
    "F64", "cfm" => "F64", "maxErrorDistance" => "F64", },
    "hkpBallSocketChainDataConstraintInfo" => phf_ordered_map! { "pivotInA" =>
    "Object|Vector4", "pivotInB" => "Object|Vector4", }, "hkpBallSocketConstraintAtom" =>
    phf_ordered_map! { "type" => "String", "solvingMethod" => "String", "bodiesToNotify"
    => "U64", "velocityStabilizationFactor" => "U64", "maxImpulse" => "F64",
    "inertiaStabilizationFactor" => "F64", }, "hkpBinaryAction" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" => "Pointer", "island"
    => "Pointer", "userData" => "U64", "name" => "String", "entityA" => "Pointer",
    "entityB" => "Pointer", }, "hkpBoxMotion" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "type" => "String", "deactivationIntegrateCounter"
    => "U64", "deactivationNumInactiveFrames" => "U64", "motionState" =>
    "Object|hkMotionState", "inertiaAndMassInv" => "Object|Vector4", "linearVelocity" =>
    "Object|Vector4", "angularVelocity" => "Object|Vector4", "deactivationRefPosition" =>
    "Object|Vector4", "deactivationRefOrientation" => "U64", "savedMotion" => "Pointer",
    "savedQualityTypeIndex" => "U64", "gravityFactor" => "F64", }, "hkpBoxShape" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "type" => "String", "radius" => "F64", "halfExtents" => "Object|Vector4",
    }, "hkpBreakableBody" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkpBreakableConstraintData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "atoms"
    => "Object|hkpBridgeAtoms", "constraintData" => "Pointer", "childRuntimeSize" =>
    "U64", "childNumSolverResults" => "U64", "solverResultLimit" => "F64",
    "removeWhenBroken" => "Bool", "revertBackVelocityOnBreak" => "Bool", },
    "hkpBridgeAtoms" => phf_ordered_map! { "bridgeAtom" =>
    "Object|hkpBridgeConstraintAtom", }, "hkpBridgeConstraintAtom" => phf_ordered_map! {
    "type" => "String", "buildJacobianFunc" => "Pointer", "constraintData" => "Pointer",
    }, "hkpBroadPhaseHandle" => phf_ordered_map! { "id" => "U64", }, "hkpBvShape" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "type" => "String", "boundingVolumeShape" => "Pointer", "childShape" =>
    "Object|hkpSingleShapeContainer", }, "hkpBvTreeShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "bvTreeType" => "String", }, "hkpCachingShapePhantom" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" => "Pointer",
    "userData" => "U64", "collidable" => "Object|hkpLinkedCollidable", "multiThreadCheck"
    => "Object|hkMultiThreadCheck", "name" => "String", "properties" =>
    "Array|Object|hkpProperty", "treeData" => "Pointer", "overlapListeners" =>
    "Array|Pointer", "phantomListeners" => "Array|Pointer", "motionState" =>
    "Object|hkMotionState", "collisionDetails" => "Array|Null", "orderDirty" => "Bool",
    }, "hkpCallbackConstraintMotor" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "type" => "String", "minForce" => "F64", "maxForce" =>
    "F64", "callbackFunc" => "Pointer", "callbackType" => "String", "userData0" => "U64",
    "userData1" => "U64", "userData2" => "U64", }, "hkpCapsuleShape" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type"
    => "String", "radius" => "F64", "vertexA" => "Object|Vector4", "vertexB" =>
    "Object|Vector4", }, "hkpCdBody" => phf_ordered_map! { "shape" => "Pointer",
    "shapeKey" => "U64", "motion" => "Pointer", "parent" => "Pointer", },
    "hkpCenterOfMassChangerModifierConstraintAtom" => phf_ordered_map! { "type" =>
    "String", "modifierAtomSize" => "U64", "childSize" => "U64", "child" => "Pointer",
    "pad" => "U64", "displacementA" => "Object|Vector4", "displacementB" =>
    "Object|Vector4", }, "hkpCharacterControllerCinfo" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", }, "hkpCharacterMotion" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" =>
    "String", "deactivationIntegrateCounter" => "U64", "deactivationNumInactiveFrames" =>
    "U64", "motionState" => "Object|hkMotionState", "inertiaAndMassInv" =>
    "Object|Vector4", "linearVelocity" => "Object|Vector4", "angularVelocity" =>
    "Object|Vector4", "deactivationRefPosition" => "Object|Vector4",
    "deactivationRefOrientation" => "U64", "savedMotion" => "Pointer",
    "savedQualityTypeIndex" => "U64", "gravityFactor" => "F64", },
    "hkpCharacterProxyCinfo" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "position" => "Object|Vector4", "velocity" =>
    "Object|Vector4", "dynamicFriction" => "F64", "staticFriction" => "F64",
    "keepContactTolerance" => "F64", "up" => "Object|Vector4", "extraUpStaticFriction" =>
    "F64", "extraDownStaticFriction" => "F64", "shapePhantom" => "Pointer",
    "keepDistance" => "F64", "contactAngleSensitivity" => "F64", "userPlanes" => "U64",
    "maxCharacterSpeedForSolver" => "F64", "characterStrength" => "F64", "characterMass"
    => "F64", "maxSlope" => "F64", "penetrationRecoverySpeed" => "F64",
    "maxCastIterations" => "I64", "refreshManifoldInCheckSupport" => "Bool", },
    "hkpCharacterRigidBodyCinfo" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "collisionFilterInfo" => "U64", "shape" => "Pointer",
    "position" => "Object|Vector4", "rotation" => "Object|Quaternion", "mass" => "F64",
    "friction" => "F64", "maxLinearVelocity" => "F64", "allowedPenetrationDepth" =>
    "F64", "up" => "Object|Vector4", "maxSlope" => "F64", "maxForce" => "F64",
    "unweldingHeightOffsetFactor" => "F64", "maxSpeedForSimplexSolver" => "F64",
    "supportDistance" => "F64", "hardSupportDistance" => "F64", "vdbColor" => "I64", },
    "hkpCogWheelConstraintAtom" => phf_ordered_map! { "type" => "String",
    "cogWheelRadiusA" => "F64", "cogWheelRadiusB" => "F64", "isScrew" => "Bool",
    "memOffsetToInitialAngleOffset" => "I64", "memOffsetToPrevAngle" => "I64",
    "memOffsetToRevolutionCounter" => "I64", }, "hkpCogWheelConstraintData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "atoms" => "Object|hkpCogWheelConstraintDataAtoms", },
    "hkpCogWheelConstraintDataAtoms" => phf_ordered_map! { "transforms" =>
    "Object|hkpSetLocalTransformsConstraintAtom", "cogWheels" =>
    "Object|hkpCogWheelConstraintAtom", }, "hkpCollidable" => phf_ordered_map! { "shape"
    => "Pointer", "shapeKey" => "U64", "motion" => "Pointer", "parent" => "Pointer",
    "ownerOffset" => "I64", "forceCollideOntoPpu" => "U64", "shapeSizeOnSpu" => "U64",
    "broadPhaseHandle" => "Object|hkpTypedBroadPhaseHandle", "boundingVolumeData" =>
    "Object|hkpCollidableBoundingVolumeData", "allowedPenetrationDepth" => "F64", },
    "hkpCollidableBoundingVolumeData" => phf_ordered_map! { "min" => "U64",
    "expansionMin" => "U64", "expansionShift" => "U64", "max" => "U64", "expansionMax" =>
    "U64", "padding" => "U64", "numChildShapeAabbs" => "U64", "capacityChildShapeAabbs"
    => "U64", "childShapeAabbs" => "Pointer", "childShapeKeys" => "Pointer", },
    "hkpCollidableCollidableFilter" => phf_ordered_map! {}, "hkpCollisionFilter" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "prepad" =>
    "U64", "type" => "String", "postpad" => "U64", }, "hkpCollisionFilterList" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "prepad" =>
    "U64", "type" => "String", "postpad" => "U64", "collisionFilters" => "Array|Pointer",
    }, "hkpCompressedMeshShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "disableWelding"
    => "Bool", "collectionType" => "String", "bitsPerIndex" => "I64", "bitsPerWIndex" =>
    "I64", "wIndexMask" => "I64", "indexMask" => "I64", "radius" => "F64", "weldingType"
    => "String", "materialType" => "String", "materials" => "Array|U64", "materials16" =>
    "Array|U64", "materials8" => "Array|U64", "transforms" => "Array|Object|QsTransform",
    "bigVertices" => "Array|Object|Vector4", "bigTriangles" =>
    "Array|Object|hkpCompressedMeshShapeBigTriangle", "chunks" =>
    "Array|Object|hkpCompressedMeshShapeChunk", "convexPieces" =>
    "Array|Object|hkpCompressedMeshShapeConvexPiece", "error" => "F64", "bounds" =>
    "Object|hkAabb", "defaultCollisionFilterInfo" => "U64", "meshMaterials" => "Pointer",
    "materialStriding" => "U64", "numMaterials" => "U64", "namedMaterials" =>
    "Array|Object|hkpNamedMeshMaterial", }, "hkpCompressedMeshShapeBigTriangle" =>
    phf_ordered_map! { "a" => "U64", "b" => "U64", "c" => "U64", "material" => "U64",
    "weldingInfo" => "U64", "transformIndex" => "U64", }, "hkpCompressedMeshShapeChunk"
    => phf_ordered_map! { "offset" => "Object|Vector4", "vertices" => "Array|U64",
    "indices" => "Array|U64", "stripLengths" => "Array|U64", "weldingInfo" =>
    "Array|U64", "materialInfo" => "U64", "reference" => "U64", "transformIndex" =>
    "U64", }, "hkpCompressedMeshShapeConvexPiece" => phf_ordered_map! { "offset" =>
    "Object|Vector4", "vertices" => "Array|U64", "faceVertices" => "Array|U64",
    "faceOffsets" => "Array|U64", "reference" => "U64", "transformIndex" => "U64", },
    "hkpCompressedSampledHeightFieldShape" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "userData" => "U64", "type" => "String", "xRes" =>
    "I64", "zRes" => "I64", "heightCenter" => "F64", "useProjectionBasedHeight" =>
    "Bool", "heightfieldType" => "String", "intToFloatScale" => "Object|Vector4",
    "floatToIntScale" => "Object|Vector4", "floatToIntOffsetFloorCorrected" =>
    "Object|Vector4", "extents" => "Object|Vector4", "storage" => "Array|U64",
    "triangleFlip" => "Bool", "offset" => "F64", "scale" => "F64", },
    "hkpConeLimitConstraintAtom" => phf_ordered_map! { "type" => "String", "isEnabled" =>
    "U64", "twistAxisInA" => "U64", "refAxisInB" => "U64", "angleMeasurementMode" =>
    "String", "memOffsetToAngleOffset" => "U64", "minAngle" => "F64", "maxAngle" =>
    "F64", "angularLimitsTauFactor" => "F64", }, "hkpConstrainedSystemFilter" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "prepad" =>
    "U64", "type" => "String", "postpad" => "U64", "otherFilter" => "Pointer", },
    "hkpConstraintAtom" => phf_ordered_map! { "type" => "String", },
    "hkpConstraintChainData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", }, "hkpConstraintChainInstance" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "owner" =>
    "Pointer", "data" => "Pointer", "constraintModifiers" => "Pointer", "entities" =>
    "Pointer", "priority" => "String", "wantRuntime" => "Bool", "destructionRemapInfo" =>
    "String", "listeners" =>
    "Object|hkpConstraintInstanceSmallArraySerializeOverrideType", "name" => "String",
    "userData" => "U64", "internal" => "Pointer", "uid" => "U64", "chainedEntities" =>
    "Array|Pointer", "action" => "Pointer", }, "hkpConstraintChainInstanceAction" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" =>
    "Pointer", "island" => "Pointer", "userData" => "U64", "name" => "String",
    "constraintInstance" => "Pointer", }, "hkpConstraintCollisionFilter" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "prepad" =>
    "U64", "type" => "String", "postpad" => "U64", "disabledPairs" =>
    "Object|hkpPairCollisionFilterMapPairFilterKeyOverrideType", "childFilter" =>
    "Pointer", }, "hkpConstraintData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", }, "hkpConstraintInstance" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "owner" =>
    "Pointer", "data" => "Pointer", "constraintModifiers" => "Pointer", "entities" =>
    "Pointer", "priority" => "String", "wantRuntime" => "Bool", "destructionRemapInfo" =>
    "String", "listeners" =>
    "Object|hkpConstraintInstanceSmallArraySerializeOverrideType", "name" => "String",
    "userData" => "U64", "internal" => "Pointer", "uid" => "U64", },
    "hkpConstraintInstanceSmallArraySerializeOverrideType" => phf_ordered_map! { "data"
    => "Pointer", "size" => "U64", "capacityAndFlags" => "U64", }, "hkpConstraintMotor"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type"
    => "String", }, "hkpConvexListFilter" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", }, "hkpConvexListShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "radius" => "F64", "minDistanceToUseConvexHullForGetClosestPoints" =>
    "F64", "aabbHalfExtents" => "Object|Vector4", "aabbCenter" => "Object|Vector4",
    "useCachedAabb" => "Bool", "childShapes" => "Array|Pointer", },
    "hkpConvexPieceMeshShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "disableWelding"
    => "Bool", "collectionType" => "String", "convexPieceStream" => "Pointer",
    "displayMesh" => "Pointer", "radius" => "F64", }, "hkpConvexPieceStreamData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "convexPieceStream" => "Array|U64", "convexPieceOffsets" => "Array|U64",
    "convexPieceSingleTriangles" => "Array|U64", }, "hkpConvexShape" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type"
    => "String", "radius" => "F64", }, "hkpConvexTransformShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "radius" => "F64", "childShape" => "Object|hkpSingleShapeContainer",
    "childShapeSize" => "I64", "transform" => "Object|Transform", },
    "hkpConvexTransformShapeBase" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "radius" =>
    "F64", "childShape" => "Object|hkpSingleShapeContainer", "childShapeSize" => "I64",
    }, "hkpConvexTranslateShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "radius" =>
    "F64", "childShape" => "Object|hkpSingleShapeContainer", "childShapeSize" => "I64",
    "translation" => "Object|Vector4", }, "hkpConvexVerticesConnectivity" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "vertexIndices" => "Array|U64", "numVerticesPerFace" => "Array|U64", },
    "hkpConvexVerticesShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "radius" =>
    "F64", "aabbHalfExtents" => "Object|Vector4", "aabbCenter" => "Object|Vector4",
    "rotatedVertices" => "Array|Object|hkpConvexVerticesShapeFourVectors", "numVertices"
    => "I64", "externalObject" => "Pointer", "getFaceNormals" => "Pointer",
    "planeEquations" => "Array|Object|Vector4", "connectivity" => "Pointer", },
    "hkpConvexVerticesShapeFourVectors" => phf_ordered_map! { "x" => "Object|Vector4",
    "y" => "Object|Vector4", "z" => "Object|Vector4", }, "hkpCylinderShape" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "type" => "String", "radius" => "F64", "cylRadius" => "F64",
    "cylBaseRadiusFactorForHeightFieldCollisions" => "F64", "vertexA" =>
    "Object|Vector4", "vertexB" => "Object|Vector4", "perpendicular1" =>
    "Object|Vector4", "perpendicular2" => "Object|Vector4", }, "hkpDashpotAction" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" =>
    "Pointer", "island" => "Pointer", "userData" => "U64", "name" => "String", "entityA"
    => "Pointer", "entityB" => "Pointer", "point" => "Object|Vector4", "strength" =>
    "F64", "damping" => "F64", "impulse" => "Object|Vector4", },
    "hkpDefaultConvexListFilter" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkpDefaultWorldMemoryWatchDog" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "freeHeapMemoryRequested" =>
    "I64", }, "hkpDisableEntityCollisionFilter" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "prepad" => "U64", "type" => "String", "postpad"
    => "U64", "disabledEntities" => "Array|Pointer", }, "hkpDisplayBindingData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "rigidBodyBindings" => "Array|Pointer", "physicsSystemBindings" => "Array|Pointer",
    }, "hkpDisplayBindingDataPhysicsSystem" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "bindings" => "Array|Pointer", "system" =>
    "Pointer", }, "hkpDisplayBindingDataRigidBody" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "rigidBody" => "Pointer",
    "displayObjectPtr" => "Pointer", "rigidBodyFromDisplayObjectTransform" =>
    "Object|Matrix4", }, "hkpEntity" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "world" => "Pointer", "userData" => "U64", "collidable" =>
    "Object|hkpLinkedCollidable", "multiThreadCheck" => "Object|hkMultiThreadCheck",
    "name" => "String", "properties" => "Array|Object|hkpProperty", "treeData" =>
    "Pointer", "material" => "Object|hkpMaterial", "limitContactImpulseUtilAndFlag" =>
    "Pointer", "damageMultiplier" => "F64", "breakableBody" => "Pointer", "solverData" =>
    "U64", "storageIndex" => "U64", "contactPointCallbackDelay" => "U64",
    "constraintsMaster" => "Object|hkpEntitySmallArraySerializeOverrideType",
    "constraintsSlave" => "Array|Pointer", "constraintRuntime" => "Array|U64",
    "simulationIsland" => "Pointer", "autoRemoveLevel" => "I64",
    "numShapeKeysInContactPointProperties" => "U64", "responseModifierFlags" => "U64",
    "uid" => "U64", "spuCollisionCallback" => "Object|hkpEntitySpuCollisionCallback",
    "motion" => "Object|hkpMaxSizeMotion", "contactListeners" =>
    "Object|hkpEntitySmallArraySerializeOverrideType", "actions" =>
    "Object|hkpEntitySmallArraySerializeOverrideType", "localFrame" => "Pointer",
    "extendedListeners" => "Pointer", "npData" => "U64", }, "hkpEntityExtendedListeners"
    => phf_ordered_map! { "activationListeners" =>
    "Object|hkpEntitySmallArraySerializeOverrideType", "entityListeners" =>
    "Object|hkpEntitySmallArraySerializeOverrideType", },
    "hkpEntitySmallArraySerializeOverrideType" => phf_ordered_map! { "data" => "Pointer",
    "size" => "U64", "capacityAndFlags" => "U64", }, "hkpEntitySpuCollisionCallback" =>
    phf_ordered_map! { "util" => "Pointer", "capacity" => "U64", "eventFilter" => "U64",
    "userFilter" => "U64", }, "hkpExtendedMeshShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "disableWelding" => "Bool", "collectionType" => "String",
    "embeddedTrianglesSubpart" => "Object|hkpExtendedMeshShapeTrianglesSubpart",
    "aabbHalfExtents" => "Object|Vector4", "aabbCenter" => "Object|Vector4",
    "materialClass" => "Pointer", "numBitsForSubpartIndex" => "I64", "trianglesSubparts"
    => "Array|Object|hkpExtendedMeshShapeTrianglesSubpart", "shapesSubparts" =>
    "Array|Object|hkpExtendedMeshShapeShapesSubpart", "weldingInfo" => "Array|U64",
    "weldingType" => "String", "defaultCollisionFilterInfo" => "U64",
    "cachedNumChildShapes" => "I64", "triangleRadius" => "F64", "padding" => "I64", },
    "hkpExtendedMeshShapeShapesSubpart" => phf_ordered_map! { "type" => "String",
    "materialIndexStridingType" => "String", "materialStriding" => "I64",
    "materialIndexBase" => "Pointer", "materialIndexStriding" => "U64", "numMaterials" =>
    "U64", "materialBase" => "Pointer", "userData" => "U64", "childShapes" =>
    "Array|Pointer", "rotation" => "Object|Quaternion", "translation" =>
    "Object|Vector4", }, "hkpExtendedMeshShapeSubpart" => phf_ordered_map! { "type" =>
    "String", "materialIndexStridingType" => "String", "materialStriding" => "I64",
    "materialIndexBase" => "Pointer", "materialIndexStriding" => "U64", "numMaterials" =>
    "U64", "materialBase" => "Pointer", "userData" => "U64", },
    "hkpExtendedMeshShapeTrianglesSubpart" => phf_ordered_map! { "type" => "String",
    "materialIndexStridingType" => "String", "materialStriding" => "I64",
    "materialIndexBase" => "Pointer", "materialIndexStriding" => "U64", "numMaterials" =>
    "U64", "materialBase" => "Pointer", "userData" => "U64", "numTriangleShapes" =>
    "I64", "vertexBase" => "Pointer", "numVertices" => "I64", "indexBase" => "Pointer",
    "vertexStriding" => "U64", "triangleOffset" => "I64", "indexStriding" => "U64",
    "stridingType" => "String", "flipAlternateTriangles" => "I64", "extrusion" =>
    "Object|Vector4", "transform" => "Object|QsTransform", }, "hkpFastMeshShape" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "type" => "String", "disableWelding" => "Bool", "collectionType" =>
    "String", "scaling" => "Object|Vector4", "numBitsForSubpartIndex" => "I64",
    "subparts" => "Array|Object|hkpMeshShapeSubpart", "weldingInfo" => "Array|U64",
    "weldingType" => "String", "radius" => "F64", "pad" => "I64", }, "hkpFirstPersonGun"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type"
    => "String", "name" => "String", "keyboardKey" => "String", "listeners" =>
    "Array|Pointer", }, "hkpFixedRigidMotion" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "type" => "String", "deactivationIntegrateCounter"
    => "U64", "deactivationNumInactiveFrames" => "U64", "motionState" =>
    "Object|hkMotionState", "inertiaAndMassInv" => "Object|Vector4", "linearVelocity" =>
    "Object|Vector4", "angularVelocity" => "Object|Vector4", "deactivationRefPosition" =>
    "Object|Vector4", "deactivationRefOrientation" => "U64", "savedMotion" => "Pointer",
    "savedQualityTypeIndex" => "U64", "gravityFactor" => "F64", },
    "hkpGenericConstraintData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "atoms" => "Object|hkpBridgeAtoms",
    "scheme" => "Object|hkpGenericConstraintDataScheme", },
    "hkpGenericConstraintDataScheme" => phf_ordered_map! { "info" =>
    "Object|hkpGenericConstraintDataSchemeConstraintInfo", "data" =>
    "Array|Object|Vector4", "commands" => "Array|I64", "modifiers" => "Array|Pointer",
    "motors" => "Array|Pointer", }, "hkpGenericConstraintDataSchemeConstraintInfo" =>
    phf_ordered_map! { "maxSizeOfSchema" => "I64", "sizeOfSchemas" => "I64",
    "numSolverResults" => "I64", "numSolverElemTemps" => "I64", }, "hkpGravityGun" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" =>
    "String", "name" => "String", "keyboardKey" => "String", "listeners" =>
    "Array|Pointer", "grabbedBodies" => "Array|Pointer", "maxNumObjectsPicked" => "I64",
    "maxMassOfObjectPicked" => "F64", "maxDistOfObjectPicked" => "F64",
    "impulseAppliedWhenObjectNotPicked" => "F64", "throwVelocity" => "F64",
    "capturedObjectPosition" => "Object|Vector4", "capturedObjectsOffset" =>
    "Object|Vector4", }, "hkpGroupCollisionFilter" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "prepad" => "U64", "type" =>
    "String", "postpad" => "U64", "noGroupCollisionEnabled" => "Bool", "collisionGroups"
    => "U64", }, "hkpGroupFilter" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "prepad" => "U64", "type" => "String", "postpad" => "U64",
    "nextFreeSystemGroup" => "I64", "collisionLookupTable" => "U64", "pad256" =>
    "Object|Vector4", }, "hkpHeightFieldShape" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "userData" => "U64", "type" => "String", },
    "hkpHingeConstraintData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "atoms" =>
    "Object|hkpHingeConstraintDataAtoms", }, "hkpHingeConstraintDataAtoms" =>
    phf_ordered_map! { "transforms" => "Object|hkpSetLocalTransformsConstraintAtom",
    "setupStabilization" => "Object|hkpSetupStabilizationAtom", "2dAng" =>
    "Object|hkp2dAngConstraintAtom", "ballSocket" =>
    "Object|hkpBallSocketConstraintAtom", }, "hkpHingeLimitsData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "atoms"
    => "Object|hkpHingeLimitsDataAtoms", }, "hkpHingeLimitsDataAtoms" => phf_ordered_map!
    { "rotations" => "Object|hkpSetLocalRotationsConstraintAtom", "angLimit" =>
    "Object|hkpAngLimitConstraintAtom", "2dAng" => "Object|hkp2dAngConstraintAtom", },
    "hkpIgnoreModifierConstraintAtom" => phf_ordered_map! { "type" => "String",
    "modifierAtomSize" => "U64", "childSize" => "U64", "child" => "Pointer", "pad" =>
    "U64", }, "hkpKeyframedRigidMotion" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "type" => "String", "deactivationIntegrateCounter" =>
    "U64", "deactivationNumInactiveFrames" => "U64", "motionState" =>
    "Object|hkMotionState", "inertiaAndMassInv" => "Object|Vector4", "linearVelocity" =>
    "Object|Vector4", "angularVelocity" => "Object|Vector4", "deactivationRefPosition" =>
    "Object|Vector4", "deactivationRefOrientation" => "U64", "savedMotion" => "Pointer",
    "savedQualityTypeIndex" => "U64", "gravityFactor" => "F64", },
    "hkpLimitedForceConstraintMotor" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "type" => "String", "minForce" => "F64", "maxForce" =>
    "F64", }, "hkpLimitedHingeConstraintData" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "userData" => "U64", "atoms" =>
    "Object|hkpLimitedHingeConstraintDataAtoms", }, "hkpLimitedHingeConstraintDataAtoms"
    => phf_ordered_map! { "transforms" => "Object|hkpSetLocalTransformsConstraintAtom",
    "setupStabilization" => "Object|hkpSetupStabilizationAtom", "angMotor" =>
    "Object|hkpAngMotorConstraintAtom", "angFriction" =>
    "Object|hkpAngFrictionConstraintAtom", "angLimit" =>
    "Object|hkpAngLimitConstraintAtom", "2dAng" => "Object|hkp2dAngConstraintAtom",
    "ballSocket" => "Object|hkpBallSocketConstraintAtom", }, "hkpLinConstraintAtom" =>
    phf_ordered_map! { "type" => "String", "axisIndex" => "U64", },
    "hkpLinFrictionConstraintAtom" => phf_ordered_map! { "type" => "String", "isEnabled"
    => "U64", "frictionAxis" => "U64", "maxFrictionForce" => "F64", },
    "hkpLinLimitConstraintAtom" => phf_ordered_map! { "type" => "String", "axisIndex" =>
    "U64", "min" => "F64", "max" => "F64", }, "hkpLinMotorConstraintAtom" =>
    phf_ordered_map! { "type" => "String", "isEnabled" => "Bool", "motorAxis" => "U64",
    "initializedOffset" => "I64", "previousTargetPositionOffset" => "I64",
    "targetPosition" => "F64", "motor" => "Pointer", }, "hkpLinSoftConstraintAtom" =>
    phf_ordered_map! { "type" => "String", "axisIndex" => "U64", "tau" => "F64",
    "damping" => "F64", }, "hkpLinearParametricCurve" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "smoothingFactor" => "F64",
    "closedLoop" => "Bool", "dirNotParallelToTangentAlongWholePath" => "Object|Vector4",
    "points" => "Array|Object|Vector4", "distance" => "Array|F64", },
    "hkpLinkedCollidable" => phf_ordered_map! { "shape" => "Pointer", "shapeKey" =>
    "U64", "motion" => "Pointer", "parent" => "Pointer", "ownerOffset" => "I64",
    "forceCollideOntoPpu" => "U64", "shapeSizeOnSpu" => "U64", "broadPhaseHandle" =>
    "Object|hkpTypedBroadPhaseHandle", "boundingVolumeData" =>
    "Object|hkpCollidableBoundingVolumeData", "allowedPenetrationDepth" => "F64",
    "collisionEntries" => "Array|Null", }, "hkpListShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "disableWelding" => "Bool", "collectionType" => "String", "childInfo" =>
    "Array|Object|hkpListShapeChildInfo", "flags" => "U64", "numDisabledChildren" =>
    "U64", "aabbHalfExtents" => "Object|Vector4", "aabbCenter" => "Object|Vector4",
    "enabledChildren" => "U64", }, "hkpListShapeChildInfo" => phf_ordered_map! { "shape"
    => "Pointer", "collisionFilterInfo" => "U64", "shapeSize" => "I64", "numChildShapes"
    => "I64", }, "hkpMalleableConstraintData" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "userData" => "U64", "constraintData" => "Pointer",
    "atoms" => "Object|hkpBridgeAtoms", "strength" => "F64", },
    "hkpMassChangerModifierConstraintAtom" => phf_ordered_map! { "type" => "String",
    "modifierAtomSize" => "U64", "childSize" => "U64", "child" => "Pointer", "pad" =>
    "U64", "factorA" => "Object|Vector4", "factorB" => "Object|Vector4", },
    "hkpMassProperties" => phf_ordered_map! { "volume" => "F64", "mass" => "F64",
    "centerOfMass" => "Object|Vector4", "inertiaTensor" => "Object|Matrix3", },
    "hkpMaterial" => phf_ordered_map! { "responseType" => "String",
    "rollingFrictionMultiplier" => "F64", "friction" => "F64", "restitution" => "F64", },
    "hkpMaxSizeMotion" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "type" => "String", "deactivationIntegrateCounter" => "U64",
    "deactivationNumInactiveFrames" => "U64", "motionState" => "Object|hkMotionState",
    "inertiaAndMassInv" => "Object|Vector4", "linearVelocity" => "Object|Vector4",
    "angularVelocity" => "Object|Vector4", "deactivationRefPosition" => "Object|Vector4",
    "deactivationRefOrientation" => "U64", "savedMotion" => "Pointer",
    "savedQualityTypeIndex" => "U64", "gravityFactor" => "F64", }, "hkpMeshMaterial" =>
    phf_ordered_map! { "filterInfo" => "U64", }, "hkpMeshShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "disableWelding" => "Bool", "collectionType" => "String", "scaling" =>
    "Object|Vector4", "numBitsForSubpartIndex" => "I64", "subparts" =>
    "Array|Object|hkpMeshShapeSubpart", "weldingInfo" => "Array|U64", "weldingType" =>
    "String", "radius" => "F64", "pad" => "I64", }, "hkpMeshShapeSubpart" =>
    phf_ordered_map! { "vertexBase" => "Pointer", "vertexStriding" => "I64",
    "numVertices" => "I64", "indexBase" => "Pointer", "stridingType" => "String",
    "materialIndexStridingType" => "String", "indexStriding" => "I64",
    "flipAlternateTriangles" => "I64", "numTriangles" => "I64", "materialIndexBase" =>
    "Pointer", "materialIndexStriding" => "I64", "materialBase" => "Pointer",
    "materialStriding" => "I64", "numMaterials" => "I64", "triangleOffset" => "I64", },
    "hkpModifierConstraintAtom" => phf_ordered_map! { "type" => "String",
    "modifierAtomSize" => "U64", "childSize" => "U64", "child" => "Pointer", "pad" =>
    "U64", }, "hkpMoppBvTreeShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "bvTreeType" =>
    "String", "code" => "Pointer", "moppData" => "Pointer", "moppDataSize" => "U64",
    "codeInfoCopy" => "Object|Vector4", "child" => "Object|hkpSingleShapeContainer",
    "childSize" => "I64", }, "hkpMoppCode" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "info" => "Object|hkpMoppCodeCodeInfo", "data" =>
    "Array|U64", "buildType" => "String", }, "hkpMoppCodeCodeInfo" => phf_ordered_map! {
    "offset" => "Object|Vector4", }, "hkpMoppCodeReindexedTerminal" => phf_ordered_map! {
    "origShapeKey" => "U64", "reindexedShapeKey" => "U64", }, "hkpMotion" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" =>
    "String", "deactivationIntegrateCounter" => "U64", "deactivationNumInactiveFrames" =>
    "U64", "motionState" => "Object|hkMotionState", "inertiaAndMassInv" =>
    "Object|Vector4", "linearVelocity" => "Object|Vector4", "angularVelocity" =>
    "Object|Vector4", "deactivationRefPosition" => "Object|Vector4",
    "deactivationRefOrientation" => "U64", "savedMotion" => "Pointer",
    "savedQualityTypeIndex" => "U64", "gravityFactor" => "F64", }, "hkpMotorAction" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" =>
    "Pointer", "island" => "Pointer", "userData" => "U64", "name" => "String", "entity"
    => "Pointer", "axis" => "Object|Vector4", "spinRate" => "F64", "gain" => "F64",
    "active" => "Bool", }, "hkpMountedBallGun" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "type" => "String", "name" => "String",
    "keyboardKey" => "String", "listeners" => "Array|Pointer", "bulletRadius" => "F64",
    "bulletVelocity" => "F64", "bulletMass" => "F64", "damageMultiplier" => "F64",
    "maxBulletsInWorld" => "I64", "bulletOffsetFromCenter" => "Object|Vector4",
    "addedBodies" => "Pointer", "position" => "Object|Vector4", }, "hkpMouseSpringAction"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world"
    => "Pointer", "island" => "Pointer", "userData" => "U64", "name" => "String",
    "entity" => "Pointer", "positionInRbLocal" => "Object|Vector4",
    "mousePositionInWorld" => "Object|Vector4", "springDamping" => "F64",
    "springElasticity" => "F64", "maxRelativeForce" => "F64", "objectDamping" => "F64",
    "shapeKey" => "U64", "applyCallbacks" => "Array|Pointer", },
    "hkpMovingSurfaceModifierConstraintAtom" => phf_ordered_map! { "type" => "String",
    "modifierAtomSize" => "U64", "childSize" => "U64", "child" => "Pointer", "pad" =>
    "U64", "velocity" => "Object|Vector4", }, "hkpMultiRayShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "rays" => "Array|Object|hkpMultiRayShapeRay", "rayPenetrationDistance" =>
    "F64", }, "hkpMultiRayShapeRay" => phf_ordered_map! { "start" => "Object|Vector4",
    "end" => "Object|Vector4", }, "hkpMultiSphereShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "numSpheres" => "I64", "spheres" => "Object|Vector4", },
    "hkpMultithreadedVehicleManager" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "registeredVehicles" => "Array|Pointer", },
    "hkpNamedMeshMaterial" => phf_ordered_map! { "filterInfo" => "U64", "name" =>
    "String", }, "hkpNullCollisionFilter" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "prepad" => "U64", "type" => "String", "postpad" =>
    "U64", }, "hkpOverwritePivotConstraintAtom" => phf_ordered_map! { "type" => "String",
    "copyToPivotBFromPivotA" => "U64", }, "hkpPairCollisionFilter" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "prepad" => "U64", "type" =>
    "String", "postpad" => "U64", "disabledPairs" =>
    "Object|hkpPairCollisionFilterMapPairFilterKeyOverrideType", "childFilter" =>
    "Pointer", }, "hkpPairCollisionFilterMapPairFilterKeyOverrideType" =>
    phf_ordered_map! { "elem" => "Pointer", "numElems" => "I64", "hashMod" => "I64", },
    "hkpParametricCurve" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkpPhantom" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "world" => "Pointer", "userData" => "U64",
    "collidable" => "Object|hkpLinkedCollidable", "multiThreadCheck" =>
    "Object|hkMultiThreadCheck", "name" => "String", "properties" =>
    "Array|Object|hkpProperty", "treeData" => "Pointer", "overlapListeners" =>
    "Array|Pointer", "phantomListeners" => "Array|Pointer", }, "hkpPhantomCallbackShape"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "userData" => "U64", "type" => "String", }, "hkpPhysicsData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "worldCinfo" => "Pointer",
    "systems" => "Array|Pointer", }, "hkpPhysicsSystem" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "rigidBodies" =>
    "Array|Pointer", "constraints" => "Array|Pointer", "actions" => "Array|Pointer",
    "phantoms" => "Array|Pointer", "name" => "String", "userData" => "U64", "active" =>
    "Bool", }, "hkpPhysicsSystemWithContacts" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "rigidBodies" => "Array|Pointer", "constraints" =>
    "Array|Pointer", "actions" => "Array|Pointer", "phantoms" => "Array|Pointer", "name"
    => "String", "userData" => "U64", "active" => "Bool", "contacts" => "Array|Pointer",
    }, "hkpPlaneShape" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "userData" => "U64", "type" => "String", "plane" => "Object|Vector4",
    "aabbCenter" => "Object|Vector4", "aabbHalfExtents" => "Object|Vector4", },
    "hkpPointToPathConstraintData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "atoms" => "Object|hkpBridgeAtoms",
    "path" => "Pointer", "maxFrictionForce" => "F64", "angularConstrainedDOF" =>
    "String", "transform_OS_KS" => "Object|Transform", }, "hkpPointToPlaneConstraintData"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "userData" => "U64", "atoms" => "Object|hkpPointToPlaneConstraintDataAtoms", },
    "hkpPointToPlaneConstraintDataAtoms" => phf_ordered_map! { "transforms" =>
    "Object|hkpSetLocalTransformsConstraintAtom", "lin" => "Object|hkpLinConstraintAtom",
    }, "hkpPositionConstraintMotor" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "type" => "String", "minForce" => "F64", "maxForce" =>
    "F64", "tau" => "F64", "damping" => "F64", "proportionalRecoveryVelocity" => "F64",
    "constantRecoveryVelocity" => "F64", }, "hkpPoweredChainData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "atoms"
    => "Object|hkpBridgeAtoms", "infos" =>
    "Array|Object|hkpPoweredChainDataConstraintInfo", "tau" => "F64", "damping" => "F64",
    "cfmLinAdd" => "F64", "cfmLinMul" => "F64", "cfmAngAdd" => "F64", "cfmAngMul" =>
    "F64", "maxErrorDistance" => "F64", }, "hkpPoweredChainDataConstraintInfo" =>
    phf_ordered_map! { "pivotInA" => "Object|Vector4", "pivotInB" => "Object|Vector4",
    "aTc" => "Object|Quaternion", "bTc" => "Object|Quaternion", "motors" => "Pointer",
    "switchBodies" => "Bool", }, "hkpPoweredChainMapper" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "links" =>
    "Array|Object|hkpPoweredChainMapperLinkInfo", "targets" =>
    "Array|Object|hkpPoweredChainMapperTarget", "chains" => "Array|Pointer", },
    "hkpPoweredChainMapperLinkInfo" => phf_ordered_map! { "firstTargetIdx" => "I64",
    "numTargets" => "I64", "limitConstraint" => "Pointer", },
    "hkpPoweredChainMapperTarget" => phf_ordered_map! { "chain" => "Pointer", "infoIndex"
    => "I64", }, "hkpPrismaticConstraintData" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "userData" => "U64", "atoms" =>
    "Object|hkpPrismaticConstraintDataAtoms", }, "hkpPrismaticConstraintDataAtoms" =>
    phf_ordered_map! { "transforms" => "Object|hkpSetLocalTransformsConstraintAtom",
    "motor" => "Object|hkpLinMotorConstraintAtom", "friction" =>
    "Object|hkpLinFrictionConstraintAtom", "ang" => "Object|hkpAngConstraintAtom", "lin0"
    => "Object|hkpLinConstraintAtom", "lin1" => "Object|hkpLinConstraintAtom", "linLimit"
    => "Object|hkpLinLimitConstraintAtom", }, "hkpProjectileGun" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" => "String", "name" =>
    "String", "keyboardKey" => "String", "listeners" => "Array|Pointer", "maxProjectiles"
    => "I64", "reloadTime" => "F64", "reload" => "F64", "projectiles" => "Array|Pointer",
    "world" => "Pointer", "destructionWorld" => "Pointer", }, "hkpProperty" =>
    phf_ordered_map! { "key" => "U64", "alignmentPadding" => "U64", "value" =>
    "Object|hkpPropertyValue", }, "hkpPropertyValue" => phf_ordered_map! { "data" =>
    "U64", }, "hkpPulleyConstraintAtom" => phf_ordered_map! { "type" => "String",
    "fixedPivotAinWorld" => "Object|Vector4", "fixedPivotBinWorld" => "Object|Vector4",
    "ropeLength" => "F64", "leverageOnBodyB" => "F64", }, "hkpPulleyConstraintData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "atoms" => "Object|hkpPulleyConstraintDataAtoms", },
    "hkpPulleyConstraintDataAtoms" => phf_ordered_map! { "translations" =>
    "Object|hkpSetLocalTranslationsConstraintAtom", "pulley" =>
    "Object|hkpPulleyConstraintAtom", }, "hkpRackAndPinionConstraintAtom" =>
    phf_ordered_map! { "type" => "String", "pinionRadiusOrScrewPitch" => "F64", "isScrew"
    => "Bool", "memOffsetToInitialAngleOffset" => "I64", "memOffsetToPrevAngle" => "I64",
    "memOffsetToRevolutionCounter" => "I64", }, "hkpRackAndPinionConstraintData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "atoms" => "Object|hkpRackAndPinionConstraintDataAtoms", },
    "hkpRackAndPinionConstraintDataAtoms" => phf_ordered_map! { "transforms" =>
    "Object|hkpSetLocalTransformsConstraintAtom", "rackAndPinion" =>
    "Object|hkpRackAndPinionConstraintAtom", }, "hkpRagdollConstraintData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "atoms" => "Object|hkpRagdollConstraintDataAtoms", },
    "hkpRagdollConstraintDataAtoms" => phf_ordered_map! { "transforms" =>
    "Object|hkpSetLocalTransformsConstraintAtom", "setupStabilization" =>
    "Object|hkpSetupStabilizationAtom", "ragdollMotors" =>
    "Object|hkpRagdollMotorConstraintAtom", "angFriction" =>
    "Object|hkpAngFrictionConstraintAtom", "twistLimit" =>
    "Object|hkpTwistLimitConstraintAtom", "coneLimit" =>
    "Object|hkpConeLimitConstraintAtom", "planesLimit" =>
    "Object|hkpConeLimitConstraintAtom", "ballSocket" =>
    "Object|hkpBallSocketConstraintAtom", }, "hkpRagdollLimitsData" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "atoms"
    => "Object|hkpRagdollLimitsDataAtoms", }, "hkpRagdollLimitsDataAtoms" =>
    phf_ordered_map! { "rotations" => "Object|hkpSetLocalRotationsConstraintAtom",
    "twistLimit" => "Object|hkpTwistLimitConstraintAtom", "coneLimit" =>
    "Object|hkpConeLimitConstraintAtom", "planesLimit" =>
    "Object|hkpConeLimitConstraintAtom", }, "hkpRagdollMotorConstraintAtom" =>
    phf_ordered_map! { "type" => "String", "isEnabled" => "Bool", "initializedOffset" =>
    "I64", "previousTargetAnglesOffset" => "I64", "target_bRca" => "Object|Matrix3",
    "motors" => "Pointer", }, "hkpRayCollidableFilter" => phf_ordered_map! {},
    "hkpRayShapeCollectionFilter" => phf_ordered_map! {}, "hkpRejectChassisListener" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "chassis"
    => "Pointer", }, "hkpRemoveTerminalsMoppModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "removeInfo" => "Array|U64",
    "tempShapesToRemove" => "Pointer", }, "hkpReorientAction" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" => "Pointer", "island"
    => "Pointer", "userData" => "U64", "name" => "String", "entity" => "Pointer",
    "rotationAxis" => "Object|Vector4", "upAxis" => "Object|Vector4", "strength" =>
    "F64", "damping" => "F64", }, "hkpRigidBody" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "world" => "Pointer", "userData" => "U64",
    "collidable" => "Object|hkpLinkedCollidable", "multiThreadCheck" =>
    "Object|hkMultiThreadCheck", "name" => "String", "properties" =>
    "Array|Object|hkpProperty", "treeData" => "Pointer", "material" =>
    "Object|hkpMaterial", "limitContactImpulseUtilAndFlag" => "Pointer",
    "damageMultiplier" => "F64", "breakableBody" => "Pointer", "solverData" => "U64",
    "storageIndex" => "U64", "contactPointCallbackDelay" => "U64", "constraintsMaster" =>
    "Object|hkpEntitySmallArraySerializeOverrideType", "constraintsSlave" =>
    "Array|Pointer", "constraintRuntime" => "Array|U64", "simulationIsland" => "Pointer",
    "autoRemoveLevel" => "I64", "numShapeKeysInContactPointProperties" => "U64",
    "responseModifierFlags" => "U64", "uid" => "U64", "spuCollisionCallback" =>
    "Object|hkpEntitySpuCollisionCallback", "motion" => "Object|hkpMaxSizeMotion",
    "contactListeners" => "Object|hkpEntitySmallArraySerializeOverrideType", "actions" =>
    "Object|hkpEntitySmallArraySerializeOverrideType", "localFrame" => "Pointer",
    "extendedListeners" => "Pointer", "npData" => "U64", }, "hkpRotationalConstraintData"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "userData" => "U64", "atoms" => "Object|hkpRotationalConstraintDataAtoms", },
    "hkpRotationalConstraintDataAtoms" => phf_ordered_map! { "rotations" =>
    "Object|hkpSetLocalRotationsConstraintAtom", "ang" => "Object|hkpAngConstraintAtom",
    }, "hkpSampledHeightFieldShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "xRes" => "I64",
    "zRes" => "I64", "heightCenter" => "F64", "useProjectionBasedHeight" => "Bool",
    "heightfieldType" => "String", "intToFloatScale" => "Object|Vector4",
    "floatToIntScale" => "Object|Vector4", "floatToIntOffsetFloorCorrected" =>
    "Object|Vector4", "extents" => "Object|Vector4", }, "hkpSerializedAgentNnEntry" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "bodyA" =>
    "Pointer", "bodyB" => "Pointer", "bodyAId" => "U64", "bodyBId" => "U64",
    "useEntityIds" => "Bool", "agentType" => "String", "atom" =>
    "Object|hkpSimpleContactConstraintAtom", "propertiesStream" => "Array|U64",
    "contactPoints" => "Array|Object|hkContactPoint", "cpIdMgr" => "Array|U64",
    "nnEntryData" => "U64", "trackInfo" => "Object|hkpSerializedTrack1nInfo",
    "endianCheckBuffer" => "U64", "version" => "U64", }, "hkpSerializedDisplayMarker" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "transform"
    => "Object|Transform", }, "hkpSerializedDisplayMarkerList" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "markers" => "Array|Pointer",
    }, "hkpSerializedDisplayRbTransforms" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "transforms" =>
    "Array|Object|hkpSerializedDisplayRbTransformsDisplayTransformPair", },
    "hkpSerializedDisplayRbTransformsDisplayTransformPair" => phf_ordered_map! { "rb" =>
    "Pointer", "localToDisplay" => "Object|Transform", }, "hkpSerializedSubTrack1nInfo"
    => phf_ordered_map! { "sectors" => "Array|Pointer", "subTracks" => "Array|Pointer",
    "sectorIndex" => "I64", "offsetInSector" => "I64", }, "hkpSerializedTrack1nInfo" =>
    phf_ordered_map! { "sectors" => "Array|Pointer", "subTracks" => "Array|Pointer", },
    "hkpSetLocalRotationsConstraintAtom" => phf_ordered_map! { "type" => "String",
    "rotationA" => "Object|Rotation", "rotationB" => "Object|Rotation", },
    "hkpSetLocalTransformsConstraintAtom" => phf_ordered_map! { "type" => "String",
    "transformA" => "Object|Transform", "transformB" => "Object|Transform", },
    "hkpSetLocalTranslationsConstraintAtom" => phf_ordered_map! { "type" => "String",
    "translationA" => "Object|Vector4", "translationB" => "Object|Vector4", },
    "hkpSetupStabilizationAtom" => phf_ordered_map! { "type" => "String", "enabled" =>
    "Bool", "maxAngle" => "F64", "padding" => "U64", }, "hkpShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", }, "hkpShapeCollection" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "disableWelding"
    => "Bool", "collectionType" => "String", }, "hkpShapeCollectionFilter" =>
    phf_ordered_map! {}, "hkpShapeContainer" => phf_ordered_map! {}, "hkpShapeInfo" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "shape" =>
    "Pointer", "isHierarchicalCompound" => "Bool", "hkdShapesCollected" => "Bool",
    "childShapeNames" => "Array|String", "childTransforms" => "Array|Object|Transform",
    "transform" => "Object|Transform", }, "hkpShapeModifier" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", }, "hkpShapePhantom" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" =>
    "Pointer", "userData" => "U64", "collidable" => "Object|hkpLinkedCollidable",
    "multiThreadCheck" => "Object|hkMultiThreadCheck", "name" => "String", "properties"
    => "Array|Object|hkpProperty", "treeData" => "Pointer", "overlapListeners" =>
    "Array|Pointer", "phantomListeners" => "Array|Pointer", "motionState" =>
    "Object|hkMotionState", }, "hkpSimpleContactConstraintAtom" => phf_ordered_map! {
    "type" => "String", "sizeOfAllAtoms" => "U64", "numContactPoints" => "U64",
    "numReservedContactPoints" => "U64", "numUserDatasForBodyA" => "U64",
    "numUserDatasForBodyB" => "U64", "contactPointPropertiesStriding" => "U64",
    "maxNumContactPoints" => "U64", "info" =>
    "Object|hkpSimpleContactConstraintDataInfo", }, "hkpSimpleContactConstraintDataInfo"
    => phf_ordered_map! { "flags" => "U64", "index" => "U64", "internalData0" => "F64",
    "rollingFrictionMultiplier" => "F64", "internalData1" => "F64", "data" => "U64", },
    "hkpSimpleMeshShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "disableWelding"
    => "Bool", "collectionType" => "String", "vertices" => "Array|Object|Vector4",
    "triangles" => "Array|Object|hkpSimpleMeshShapeTriangle", "materialIndices" =>
    "Array|U64", "radius" => "F64", "weldingType" => "String", },
    "hkpSimpleMeshShapeTriangle" => phf_ordered_map! { "a" => "I64", "b" => "I64", "c" =>
    "I64", "weldingInfo" => "U64", }, "hkpSimpleShapePhantom" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" => "Pointer",
    "userData" => "U64", "collidable" => "Object|hkpLinkedCollidable", "multiThreadCheck"
    => "Object|hkMultiThreadCheck", "name" => "String", "properties" =>
    "Array|Object|hkpProperty", "treeData" => "Pointer", "overlapListeners" =>
    "Array|Pointer", "phantomListeners" => "Array|Pointer", "motionState" =>
    "Object|hkMotionState", "collisionDetails" =>
    "Array|Object|hkpSimpleShapePhantomCollisionDetail", "orderDirty" => "Bool", },
    "hkpSimpleShapePhantomCollisionDetail" => phf_ordered_map! { "collidable" =>
    "Pointer", }, "hkpSimulation" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "determinismCheckFrameCounter" => "U64", "world" =>
    "Pointer", "lastProcessingStep" => "String", "currentTime" => "F64", "currentPsiTime"
    => "F64", "physicsDeltaTime" => "F64", "simulateUntilTime" => "F64",
    "frameMarkerPsiSnap" => "F64", "previousSteModalResult" => "U64", },
    "hkpSingleShapeContainer" => phf_ordered_map! { "childShape" => "Pointer", },
    "hkpSoftContactModifierConstraintAtom" => phf_ordered_map! { "type" => "String",
    "modifierAtomSize" => "U64", "childSize" => "U64", "child" => "Pointer", "pad" =>
    "U64", "tau" => "F64", "maxAcceleration" => "F64", }, "hkpSphereMotion" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" =>
    "String", "deactivationIntegrateCounter" => "U64", "deactivationNumInactiveFrames" =>
    "U64", "motionState" => "Object|hkMotionState", "inertiaAndMassInv" =>
    "Object|Vector4", "linearVelocity" => "Object|Vector4", "angularVelocity" =>
    "Object|Vector4", "deactivationRefPosition" => "Object|Vector4",
    "deactivationRefOrientation" => "U64", "savedMotion" => "Pointer",
    "savedQualityTypeIndex" => "U64", "gravityFactor" => "F64", }, "hkpSphereRepShape" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "type" => "String", }, "hkpSphereShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "radius" => "F64", "pad16" => "U64", }, "hkpSpringAction" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" =>
    "Pointer", "island" => "Pointer", "userData" => "U64", "name" => "String", "entityA"
    => "Pointer", "entityB" => "Pointer", "lastForce" => "Object|Vector4", "positionAinA"
    => "Object|Vector4", "positionBinB" => "Object|Vector4", "restLength" => "F64",
    "strength" => "F64", "damping" => "F64", "onCompression" => "Bool", "onExtension" =>
    "Bool", }, "hkpSpringDamperConstraintMotor" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "type" => "String", "minForce" => "F64",
    "maxForce" => "F64", "springConstant" => "F64", "springDamping" => "F64", },
    "hkpStiffSpringChainData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "atoms" => "Object|hkpBridgeAtoms",
    "infos" => "Array|Object|hkpStiffSpringChainDataConstraintInfo", "tau" => "F64",
    "damping" => "F64", "cfm" => "F64", }, "hkpStiffSpringChainDataConstraintInfo" =>
    phf_ordered_map! { "pivotInA" => "Object|Vector4", "pivotInB" => "Object|Vector4",
    "springLength" => "F64", }, "hkpStiffSpringConstraintAtom" => phf_ordered_map! {
    "type" => "String", "length" => "F64", }, "hkpStiffSpringConstraintData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "atoms" => "Object|hkpStiffSpringConstraintDataAtoms", },
    "hkpStiffSpringConstraintDataAtoms" => phf_ordered_map! { "pivots" =>
    "Object|hkpSetLocalTranslationsConstraintAtom", "spring" =>
    "Object|hkpStiffSpringConstraintAtom", }, "hkpStorageExtendedMeshShape" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "type" => "String", "disableWelding" => "Bool", "collectionType" =>
    "String", "embeddedTrianglesSubpart" =>
    "Object|hkpExtendedMeshShapeTrianglesSubpart", "aabbHalfExtents" => "Object|Vector4",
    "aabbCenter" => "Object|Vector4", "materialClass" => "Pointer",
    "numBitsForSubpartIndex" => "I64", "trianglesSubparts" =>
    "Array|Object|hkpExtendedMeshShapeTrianglesSubpart", "shapesSubparts" =>
    "Array|Object|hkpExtendedMeshShapeShapesSubpart", "weldingInfo" => "Array|U64",
    "weldingType" => "String", "defaultCollisionFilterInfo" => "U64",
    "cachedNumChildShapes" => "I64", "triangleRadius" => "F64", "padding" => "I64",
    "meshstorage" => "Array|Pointer", "shapestorage" => "Array|Pointer", },
    "hkpStorageExtendedMeshShapeMaterial" => phf_ordered_map! { "filterInfo" => "U64",
    "restitution" => "F64", "friction" => "F64", "userData" => "U64", },
    "hkpStorageExtendedMeshShapeMeshSubpartStorage" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "vertices" =>
    "Array|Object|Vector4", "indices8" => "Array|U64", "indices16" => "Array|U64",
    "indices32" => "Array|U64", "materialIndices" => "Array|U64", "materials" =>
    "Array|Object|hkpStorageExtendedMeshShapeMaterial", "namedMaterials" =>
    "Array|Object|hkpNamedMeshMaterial", "materialIndices16" => "Array|U64", },
    "hkpStorageExtendedMeshShapeShapeSubpartStorage" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "materialIndices" =>
    "Array|U64", "materials" => "Array|Object|hkpStorageExtendedMeshShapeMaterial",
    "materialIndices16" => "Array|U64", }, "hkpStorageMeshShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "disableWelding" => "Bool", "collectionType" => "String", "scaling" =>
    "Object|Vector4", "numBitsForSubpartIndex" => "I64", "subparts" =>
    "Array|Object|hkpMeshShapeSubpart", "weldingInfo" => "Array|U64", "weldingType" =>
    "String", "radius" => "F64", "pad" => "I64", "storage" => "Array|Pointer", },
    "hkpStorageMeshShapeSubpartStorage" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "vertices" => "Array|F64", "indices16" => "Array|U64",
    "indices32" => "Array|U64", "materialIndices" => "Array|U64", "materials" =>
    "Array|U64", "materialIndices16" => "Array|U64", },
    "hkpStorageSampledHeightFieldShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "xRes" => "I64",
    "zRes" => "I64", "heightCenter" => "F64", "useProjectionBasedHeight" => "Bool",
    "heightfieldType" => "String", "intToFloatScale" => "Object|Vector4",
    "floatToIntScale" => "Object|Vector4", "floatToIntOffsetFloorCorrected" =>
    "Object|Vector4", "extents" => "Object|Vector4", "storage" => "Array|F64",
    "triangleFlip" => "Bool", }, "hkpThinBoxMotion" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" => "String",
    "deactivationIntegrateCounter" => "U64", "deactivationNumInactiveFrames" => "U64",
    "motionState" => "Object|hkMotionState", "inertiaAndMassInv" => "Object|Vector4",
    "linearVelocity" => "Object|Vector4", "angularVelocity" => "Object|Vector4",
    "deactivationRefPosition" => "Object|Vector4", "deactivationRefOrientation" => "U64",
    "savedMotion" => "Pointer", "savedQualityTypeIndex" => "U64", "gravityFactor" =>
    "F64", }, "hkpTransformShape" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "userData" => "U64", "type" => "String", "childShape" =>
    "Object|hkpSingleShapeContainer", "childShapeSize" => "I64", "rotation" =>
    "Object|Quaternion", "transform" => "Object|Transform", },
    "hkpTriSampledHeightFieldBvTreeShape" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "userData" => "U64", "type" => "String",
    "bvTreeType" => "String", "childContainer" => "Object|hkpSingleShapeContainer",
    "childSize" => "I64", "wantAabbRejectionTest" => "Bool", "padding" => "U64", },
    "hkpTriSampledHeightFieldCollection" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "userData" => "U64", "type" => "String",
    "disableWelding" => "Bool", "collectionType" => "String", "heightfield" => "Pointer",
    "childSize" => "I64", "radius" => "F64", "weldingInfo" => "Array|U64",
    "triangleExtrusion" => "Object|Vector4", }, "hkpTriangleShape" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData" => "U64", "type" =>
    "String", "radius" => "F64", "weldingInfo" => "U64", "weldingType" => "String",
    "isExtruded" => "U64", "vertexA" => "Object|Vector4", "vertexB" => "Object|Vector4",
    "vertexC" => "Object|Vector4", "extrusion" => "Object|Vector4", }, "hkpTriggerVolume"
    => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "overlappingBodies" => "Array|Pointer", "eventQueue" =>
    "Array|Object|hkpTriggerVolumeEventInfo", "triggerBody" => "Pointer",
    "sequenceNumber" => "U64", }, "hkpTriggerVolumeEventInfo" => phf_ordered_map! {
    "sortValue" => "U64", "body" => "Pointer", "operation" => "String", },
    "hkpTwistLimitConstraintAtom" => phf_ordered_map! { "type" => "String", "isEnabled"
    => "U64", "twistAxis" => "U64", "refAxis" => "U64", "minAngle" => "F64", "maxAngle"
    => "F64", "angularLimitsTauFactor" => "F64", }, "hkpTypedBroadPhaseHandle" =>
    phf_ordered_map! { "id" => "U64", "type" => "I64", "ownerOffset" => "I64",
    "objectQualityType" => "I64", "collisionFilterInfo" => "U64", }, "hkpTyremarkPoint"
    => phf_ordered_map! { "pointLeft" => "Object|Vector4", "pointRight" =>
    "Object|Vector4", }, "hkpTyremarksInfo" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "minTyremarkEnergy" => "F64", "maxTyremarkEnergy"
    => "F64", "tyremarksWheel" => "Array|Pointer", }, "hkpTyremarksWheel" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "currentPosition" => "I64", "numPoints" => "I64", "tyremarkPoints" =>
    "Array|Object|hkpTyremarkPoint", }, "hkpUnaryAction" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "world" => "Pointer", "island"
    => "Pointer", "userData" => "U64", "name" => "String", "entity" => "Pointer", },
    "hkpVehicleAerodynamics" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkpVehicleBrake" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", },
    "hkpVehicleCastBatchingManager" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "registeredVehicles" => "Array|Pointer", "totalNumWheels"
    => "U64", }, "hkpVehicleData" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "gravity" => "Object|Vector4", "numWheels" => "I64",
    "chassisOrientation" => "Object|Rotation", "torqueRollFactor" => "F64",
    "torquePitchFactor" => "F64", "torqueYawFactor" => "F64", "extraTorqueFactor" =>
    "F64", "maxVelocityForPositionalFriction" => "F64", "chassisUnitInertiaYaw" => "F64",
    "chassisUnitInertiaRoll" => "F64", "chassisUnitInertiaPitch" => "F64",
    "frictionEqualizer" => "F64", "normalClippingAngleCos" => "F64",
    "maxFrictionSolverMassRatio" => "F64", "wheelParams" =>
    "Array|Object|hkpVehicleDataWheelComponentParams", "numWheelsPerAxle" => "Array|I64",
    "frictionDescription" => "Object|hkpVehicleFrictionDescription",
    "chassisFrictionInertiaInvDiag" => "Object|Vector4", "alreadyInitialised" => "Bool",
    }, "hkpVehicleDataWheelComponentParams" => phf_ordered_map! { "radius" => "F64",
    "mass" => "F64", "width" => "F64", "friction" => "F64", "viscosityFriction" => "F64",
    "maxFriction" => "F64", "slipAngle" => "F64", "forceFeedbackMultiplier" => "F64",
    "maxContactBodyAcceleration" => "F64", "axle" => "I64", },
    "hkpVehicleDefaultAerodynamics" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "airDensity" => "F64", "frontalArea" => "F64",
    "dragCoefficient" => "F64", "liftCoefficient" => "F64", "extraGravityws" =>
    "Object|Vector4", }, "hkpVehicleDefaultAnalogDriverInput" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "slopeChangePointX" => "F64",
    "initialSlope" => "F64", "deadZone" => "F64", "autoReverse" => "Bool", },
    "hkpVehicleDefaultBrake" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "wheelBrakingProperties" =>
    "Array|Object|hkpVehicleDefaultBrakeWheelBrakingProperties", "wheelsMinTimeToBlock"
    => "F64", }, "hkpVehicleDefaultBrakeWheelBrakingProperties" => phf_ordered_map! {
    "maxBreakingTorque" => "F64", "minPedalInputToBlock" => "F64",
    "isConnectedToHandbrake" => "Bool", }, "hkpVehicleDefaultEngine" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "minRPM" => "F64", "optRPM"
    => "F64", "maxRPM" => "F64", "maxTorque" => "F64", "torqueFactorAtMinRPM" => "F64",
    "torqueFactorAtMaxRPM" => "F64", "resistanceFactorAtMinRPM" => "F64",
    "resistanceFactorAtOptRPM" => "F64", "resistanceFactorAtMaxRPM" => "F64",
    "clutchSlipRPM" => "F64", }, "hkpVehicleDefaultSteering" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "maxSteeringAngle" => "F64",
    "maxSpeedFullSteeringAngle" => "F64", "doesWheelSteer" => "Array|Bool", },
    "hkpVehicleDefaultSuspension" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "wheelParams" =>
    "Array|Object|hkpVehicleSuspensionSuspensionWheelParameters", "wheelSpringParams" =>
    "Array|Object|hkpVehicleDefaultSuspensionWheelSpringSuspensionParameters", },
    "hkpVehicleDefaultSuspensionWheelSpringSuspensionParameters" => phf_ordered_map! {
    "strength" => "F64", "dampingCompression" => "F64", "dampingRelaxation" => "F64", },
    "hkpVehicleDefaultTransmission" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "downshiftRPM" => "F64", "upshiftRPM" => "F64",
    "primaryTransmissionRatio" => "F64", "clutchDelayTime" => "F64", "reverseGearRatio"
    => "F64", "gearsRatio" => "Array|F64", "wheelsTorqueRatio" => "Array|F64", },
    "hkpVehicleDefaultVelocityDamper" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "normalSpinDamping" => "F64", "collisionSpinDamping" =>
    "F64", "collisionThreshold" => "F64", }, "hkpVehicleDriverInput" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", },
    "hkpVehicleDriverInputAnalogStatus" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "positionX" => "F64", "positionY" => "F64",
    "handbrakeButtonPressed" => "Bool", "reverseButtonPressed" => "Bool", },
    "hkpVehicleDriverInputStatus" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkpVehicleEngine" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", },
    "hkpVehicleFrictionDescription" => phf_ordered_map! { "wheelDistance" => "F64",
    "chassisMassInv" => "F64", "axleDescr" =>
    "Object|hkpVehicleFrictionDescriptionAxisDescription", },
    "hkpVehicleFrictionDescriptionAxisDescription" => phf_ordered_map! {
    "frictionCircleYtab" => "F64", "xStep" => "F64", "xStart" => "F64",
    "wheelSurfaceInertia" => "F64", "wheelSurfaceInertiaInv" => "F64",
    "wheelChassisMassRatio" => "F64", "wheelRadius" => "F64", "wheelRadiusInv" => "F64",
    "wheelDownForceFactor" => "F64", "wheelDownForceSumFactor" => "F64", },
    "hkpVehicleFrictionStatus" => phf_ordered_map! { "axis" =>
    "Object|hkpVehicleFrictionStatusAxisStatus", }, "hkpVehicleFrictionStatusAxisStatus"
    => phf_ordered_map! { "forward_slip_velocity" => "F64", "side_slip_velocity" =>
    "F64", "skid_energy_density" => "F64", "side_force" => "F64",
    "delayed_forward_impulse" => "F64", "sideRhs" => "F64", "forwardRhs" => "F64",
    "relativeSideForce" => "F64", "relativeForwardForce" => "F64", },
    "hkpVehicleInstance" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "world" => "Pointer", "island" => "Pointer", "userData" =>
    "U64", "name" => "String", "entity" => "Pointer", "data" => "Pointer", "driverInput"
    => "Pointer", "steering" => "Pointer", "engine" => "Pointer", "transmission" =>
    "Pointer", "brake" => "Pointer", "suspension" => "Pointer", "aerodynamics" =>
    "Pointer", "wheelCollide" => "Pointer", "tyreMarks" => "Pointer", "velocityDamper" =>
    "Pointer", "wheelsInfo" => "Array|Object|hkpVehicleInstanceWheelInfo",
    "frictionStatus" => "Object|hkpVehicleFrictionStatus", "deviceStatus" => "Pointer",
    "isFixed" => "Array|Bool", "wheelsTimeSinceMaxPedalInput" => "F64", "tryingToReverse"
    => "Bool", "torque" => "F64", "rpm" => "F64", "mainSteeringAngle" => "F64",
    "wheelsSteeringAngle" => "Array|F64", "isReversing" => "Bool", "currentGear" =>
    "I64", "delayed" => "Bool", "clutchDelayCountdown" => "F64", },
    "hkpVehicleInstanceWheelInfo" => phf_ordered_map! { "contactPoint" =>
    "Object|hkContactPoint", "contactFriction" => "F64", "contactBody" => "Pointer",
    "contactShapeKey" => "U64", "hardPointWs" => "Object|Vector4", "rayEndPointWs" =>
    "Object|Vector4", "currentSuspensionLength" => "F64", "suspensionDirectionWs" =>
    "Object|Vector4", "spinAxisChassisSpace" => "Object|Vector4", "spinAxisWs" =>
    "Object|Vector4", "steeringOrientationChassisSpace" => "Object|Quaternion",
    "spinVelocity" => "F64", "spinAngle" => "F64", "skidEnergyDensity" => "F64",
    "sideForce" => "F64", "forwardSlipVelocity" => "F64", "sideSlipVelocity" => "F64", },
    "hkpVehicleLinearCastBatchingManager" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "registeredVehicles" => "Array|Pointer",
    "totalNumWheels" => "U64", }, "hkpVehicleLinearCastWheelCollide" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "alreadyUsed" => "Bool",
    "type" => "String", "wheelCollisionFilterInfo" => "U64", "wheelStates" =>
    "Array|Object|hkpVehicleLinearCastWheelCollideWheelState", "rejectChassisListener" =>
    "Object|hkpRejectChassisListener", "maxExtraPenetration" => "F64",
    "startPointTolerance" => "F64", }, "hkpVehicleLinearCastWheelCollideWheelState" =>
    phf_ordered_map! { "phantom" => "Pointer", "shape" => "Pointer", "transform" =>
    "Object|Transform", "to" => "Object|Vector4", }, "hkpVehicleManager" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "registeredVehicles" => "Array|Pointer", }, "hkpVehicleRayCastBatchingManager" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "registeredVehicles" => "Array|Pointer", "totalNumWheels" => "U64", },
    "hkpVehicleRayCastWheelCollide" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "alreadyUsed" => "Bool", "type" => "String",
    "wheelCollisionFilterInfo" => "U64", "phantom" => "Pointer",
    "rejectRayChassisListener" => "Object|hkpRejectChassisListener", },
    "hkpVehicleSteering" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkpVehicleSuspension" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "wheelParams" =>
    "Array|Object|hkpVehicleSuspensionSuspensionWheelParameters", },
    "hkpVehicleSuspensionSuspensionWheelParameters" => phf_ordered_map! {
    "hardpointChassisSpace" => "Object|Vector4", "directionChassisSpace" =>
    "Object|Vector4", "length" => "F64", }, "hkpVehicleTransmission" => phf_ordered_map!
    { "memSizeAndFlags" => "U64", "referenceCount" => "I64", },
    "hkpVehicleVelocityDamper" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", }, "hkpVehicleWheelCollide" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "alreadyUsed" => "Bool",
    "type" => "String", }, "hkpVelocityConstraintMotor" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "type" => "String", "minForce"
    => "F64", "maxForce" => "F64", "tau" => "F64", "velocityTarget" => "F64",
    "useVelocityTargetFromConstraintTargets" => "Bool", },
    "hkpViscousSurfaceModifierConstraintAtom" => phf_ordered_map! { "type" => "String",
    "modifierAtomSize" => "U64", "childSize" => "U64", "child" => "Pointer", "pad" =>
    "U64", }, "hkpWeldingUtility" => phf_ordered_map! {}, "hkpWheelConstraintData" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "userData"
    => "U64", "atoms" => "Object|hkpWheelConstraintDataAtoms", "initialAxleInB" =>
    "Object|Vector4", "initialSteeringAxisInB" => "Object|Vector4", },
    "hkpWheelConstraintDataAtoms" => phf_ordered_map! { "suspensionBase" =>
    "Object|hkpSetLocalTransformsConstraintAtom", "lin0Limit" =>
    "Object|hkpLinLimitConstraintAtom", "lin0Soft" => "Object|hkpLinSoftConstraintAtom",
    "lin1" => "Object|hkpLinConstraintAtom", "lin2" => "Object|hkpLinConstraintAtom",
    "steeringBase" => "Object|hkpSetLocalRotationsConstraintAtom", "2dAng" =>
    "Object|hkp2dAngConstraintAtom", }, "hkpWorld" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "simulation" => "Pointer",
    "gravity" => "Object|Vector4", "fixedIsland" => "Pointer", "fixedRigidBody" =>
    "Pointer", "activeSimulationIslands" => "Array|Pointer", "inactiveSimulationIslands"
    => "Array|Pointer", "dirtySimulationIslands" => "Array|Pointer", "maintenanceMgr" =>
    "Pointer", "memoryWatchDog" => "Pointer", "assertOnRunningOutOfSolverMemory" =>
    "Bool", "broadPhase" => "Pointer", "kdTreeManager" => "Pointer", "autoUpdateTree" =>
    "Bool", "broadPhaseDispatcher" => "Pointer", "phantomBroadPhaseListener" =>
    "Pointer", "entityEntityBroadPhaseListener" => "Pointer", "broadPhaseBorderListener"
    => "Pointer", "multithreadedSimulationJobData" => "Pointer", "collisionInput" =>
    "Pointer", "collisionFilter" => "Pointer", "collisionDispatcher" => "Pointer",
    "convexListFilter" => "Pointer", "pendingOperations" => "Pointer",
    "pendingOperationsCount" => "I64", "pendingBodyOperationsCount" => "I64",
    "criticalOperationsLockCount" => "I64", "criticalOperationsLockCountForPhantoms" =>
    "I64", "blockExecutingPendingOperations" => "Bool", "criticalOperationsAllowed" =>
    "Bool", "pendingOperationQueues" => "Pointer", "pendingOperationQueueCount" => "I64",
    "multiThreadCheck" => "Object|hkMultiThreadCheck", "processActionsInSingleThread" =>
    "Bool", "allowIntegrationOfIslandsWithoutConstraintsInASeparateJob" => "Bool",
    "minDesiredIslandSize" => "U64", "modifyConstraintCriticalSection" => "Pointer",
    "isLocked" => "I64", "islandDirtyListCriticalSection" => "Pointer",
    "propertyMasterLock" => "Pointer", "wantSimulationIslands" => "Bool",
    "useHybridBroadphase" => "Bool", "snapCollisionToConvexEdgeThreshold" => "F64",
    "snapCollisionToConcaveEdgeThreshold" => "F64", "enableToiWeldRejection" => "Bool",
    "wantDeactivation" => "Bool", "shouldActivateOnRigidBodyTransformChange" => "Bool",
    "deactivationReferenceDistance" => "F64", "toiCollisionResponseRotateNormal" =>
    "F64", "maxSectorsPerMidphaseCollideTask" => "I64",
    "maxSectorsPerNarrowphaseCollideTask" => "I64", "processToisMultithreaded" => "Bool",
    "maxEntriesPerToiMidphaseCollideTask" => "I64",
    "maxEntriesPerToiNarrowphaseCollideTask" => "I64",
    "maxNumToiCollisionPairsSinglethreaded" => "I64", "simulationType" => "String",
    "numToisTillAllowedPenetrationSimplifiedToi" => "F64",
    "numToisTillAllowedPenetrationToi" => "F64", "numToisTillAllowedPenetrationToiHigher"
    => "F64", "numToisTillAllowedPenetrationToiForced" => "F64", "lastEntityUid" =>
    "U64", "lastIslandUid" => "U64", "lastConstraintUid" => "U64", "phantoms" =>
    "Array|Pointer", "actionListeners" => "Array|Pointer", "entityListeners" =>
    "Array|Pointer", "phantomListeners" => "Array|Pointer", "constraintListeners" =>
    "Array|Pointer", "worldDeletionListeners" => "Array|Pointer",
    "islandActivationListeners" => "Array|Pointer", "worldPostSimulationListeners" =>
    "Array|Pointer", "worldPostIntegrateListeners" => "Array|Pointer",
    "worldPostCollideListeners" => "Array|Pointer", "islandPostIntegrateListeners" =>
    "Array|Pointer", "islandPostCollideListeners" => "Array|Pointer", "contactListeners"
    => "Array|Pointer", "contactImpulseLimitBreachedListeners" => "Array|Pointer",
    "worldExtensions" => "Array|Pointer", "violatedConstraintArray" => "Pointer",
    "broadPhaseBorder" => "Pointer", "destructionWorld" => "Pointer", "npWorld" =>
    "Pointer", "broadPhaseExtents" => "Object|Vector4", "broadPhaseNumMarkers" => "I64",
    "sizeOfToiEventQueue" => "I64", "broadPhaseQuerySize" => "I64",
    "broadPhaseUpdateSize" => "I64", "contactPointGeneration" => "String", },
    "hkpWorldCinfo" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" =>
    "I64", "gravity" => "Object|Vector4", "broadPhaseQuerySize" => "I64",
    "contactRestingVelocity" => "F64", "broadPhaseBorderBehaviour" => "String",
    "mtPostponeAndSortBroadPhaseBorderCallbacks" => "Bool", "broadPhaseWorldAabb" =>
    "Object|hkAabb", "useKdTree" => "Bool", "useMultipleTree" => "Bool", "treeUpdateType"
    => "String", "autoUpdateKdTree" => "Bool", "collisionTolerance" => "F64",
    "collisionFilter" => "Pointer", "convexListFilter" => "Pointer",
    "expectedMaxLinearVelocity" => "F64", "sizeOfToiEventQueue" => "I64",
    "expectedMinPsiDeltaTime" => "F64", "memoryWatchDog" => "Pointer",
    "broadPhaseNumMarkers" => "I64", "contactPointGeneration" => "String",
    "allowToSkipConfirmedCallbacks" => "Bool", "useHybridBroadphase" => "Bool",
    "solverTau" => "F64", "solverDamp" => "F64", "solverIterations" => "I64",
    "solverMicrosteps" => "I64", "maxConstraintViolation" => "F64",
    "forceCoherentConstraintOrderingInSolver" => "Bool",
    "snapCollisionToConvexEdgeThreshold" => "F64", "snapCollisionToConcaveEdgeThreshold"
    => "F64", "enableToiWeldRejection" => "Bool", "enableDeprecatedWelding" => "Bool",
    "iterativeLinearCastEarlyOutDistance" => "F64", "iterativeLinearCastMaxIterations" =>
    "I64", "deactivationNumInactiveFramesSelectFlag0" => "U64",
    "deactivationNumInactiveFramesSelectFlag1" => "U64", "deactivationIntegrateCounter"
    => "U64", "shouldActivateOnRigidBodyTransformChange" => "Bool",
    "deactivationReferenceDistance" => "F64", "toiCollisionResponseRotateNormal" =>
    "F64", "maxSectorsPerMidphaseCollideTask" => "I64",
    "maxSectorsPerNarrowphaseCollideTask" => "I64", "processToisMultithreaded" => "Bool",
    "maxEntriesPerToiMidphaseCollideTask" => "I64",
    "maxEntriesPerToiNarrowphaseCollideTask" => "I64",
    "maxNumToiCollisionPairsSinglethreaded" => "I64",
    "numToisTillAllowedPenetrationSimplifiedToi" => "F64",
    "numToisTillAllowedPenetrationToi" => "F64", "numToisTillAllowedPenetrationToiHigher"
    => "F64", "numToisTillAllowedPenetrationToiForced" => "F64", "enableDeactivation" =>
    "Bool", "simulationType" => "String", "enableSimulationIslands" => "Bool",
    "minDesiredIslandSize" => "U64", "processActionsInSingleThread" => "Bool",
    "allowIntegrationOfIslandsWithoutConstraintsInASeparateJob" => "Bool",
    "frameMarkerPsiSnap" => "F64", "fireCollisionCallbacks" => "Bool", },
    "hkpWorldObject" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "world" => "Pointer", "userData" => "U64", "collidable" =>
    "Object|hkpLinkedCollidable", "multiThreadCheck" => "Object|hkMultiThreadCheck",
    "name" => "String", "properties" => "Array|Object|hkpProperty", "treeData" =>
    "Pointer", }, "hkxAnimatedFloat" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "floats" => "Array|F64", "hint" => "String", },
    "hkxAnimatedMatrix" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "matrices" => "Array|Object|Matrix4", "hint" => "String",
    }, "hkxAnimatedQuaternion" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "quaternions" => "Array|Object|Quaternion", },
    "hkxAnimatedVector" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "vectors" => "Array|Object|Vector4", "hint" => "String",
    }, "hkxAttribute" => phf_ordered_map! { "name" => "String", "value" => "Pointer", },
    "hkxAttributeGroup" => phf_ordered_map! { "name" => "String", "attributes" =>
    "Array|Object|hkxAttribute", }, "hkxAttributeHolder" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "attributeGroups" =>
    "Array|Object|hkxAttributeGroup", }, "hkxCamera" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "from" => "Object|Vector4",
    "focus" => "Object|Vector4", "up" => "Object|Vector4", "fov" => "F64", "far" =>
    "F64", "near" => "F64", "leftHanded" => "Bool", }, "hkxEdgeSelectionChannel" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "selectedEdges" => "Array|I64", }, "hkxEnum" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "items" => "Array|Object|hkxEnumItem", },
    "hkxEnumItem" => phf_ordered_map! { "value" => "I64", "name" => "String", },
    "hkxEnvironment" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "variables" => "Array|Object|hkxEnvironmentVariable", },
    "hkxEnvironmentVariable" => phf_ordered_map! { "name" => "String", "value" =>
    "String", }, "hkxIndexBuffer" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "indexType" => "String", "indices16" => "Array|U64",
    "indices32" => "Array|U64", "vertexBaseOffset" => "U64", "length" => "U64", },
    "hkxLight" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" =>
    "I64", "type" => "String", "position" => "Object|Vector4", "direction" =>
    "Object|Vector4", "color" => "U64", "angle" => "F64", }, "hkxMaterial" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "attributeGroups" => "Array|Object|hkxAttributeGroup", "name" => "String", "stages"
    => "Array|Object|hkxMaterialTextureStage", "diffuseColor" => "Object|Vector4",
    "ambientColor" => "Object|Vector4", "specularColor" => "Object|Vector4",
    "emissiveColor" => "Object|Vector4", "subMaterials" => "Array|Pointer", "extraData"
    => "Pointer", "properties" => "Array|Object|hkxMaterialProperty", },
    "hkxMaterialEffect" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "name" => "String", "type" => "String", "data" =>
    "Array|U64", }, "hkxMaterialProperty" => phf_ordered_map! { "key" => "U64", "value"
    => "U64", }, "hkxMaterialShader" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "name" => "String", "type" => "String", "vertexEntryName"
    => "String", "geomEntryName" => "String", "pixelEntryName" => "String", "data" =>
    "Array|U64", }, "hkxMaterialShaderSet" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "shaders" => "Array|Pointer", },
    "hkxMaterialTextureStage" => phf_ordered_map! { "texture" => "Pointer", "usageHint"
    => "String", "tcoordChannel" => "I64", }, "hkxMesh" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "sections" => "Array|Pointer",
    "userChannelInfos" => "Array|Pointer", }, "hkxMeshSection" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "vertexBuffer" => "Pointer",
    "indexBuffers" => "Array|Pointer", "material" => "Pointer", "userChannels" =>
    "Array|Pointer", }, "hkxMeshUserChannelInfo" => phf_ordered_map! { "memSizeAndFlags"
    => "U64", "referenceCount" => "I64", "attributeGroups" =>
    "Array|Object|hkxAttributeGroup", "name" => "String", "className" => "String", },
    "hkxNode" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" =>
    "I64", "attributeGroups" => "Array|Object|hkxAttributeGroup", "name" => "String",
    "object" => "Pointer", "keyFrames" => "Array|Object|Matrix4", "children" =>
    "Array|Pointer", "annotations" => "Array|Object|hkxNodeAnnotationData",
    "userProperties" => "String", "selected" => "Bool", }, "hkxNodeAnnotationData" =>
    phf_ordered_map! { "time" => "F64", "description" => "String", },
    "hkxNodeSelectionSet" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "attributeGroups" => "Array|Object|hkxAttributeGroup",
    "selectedNodes" => "Array|Pointer", "name" => "String", }, "hkxScene" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64", "modeller"
    => "String", "asset" => "String", "sceneLength" => "F64", "rootNode" => "Pointer",
    "selectionSets" => "Array|Pointer", "cameras" => "Array|Pointer", "lights" =>
    "Array|Pointer", "meshes" => "Array|Pointer", "materials" => "Array|Pointer",
    "inplaceTextures" => "Array|Pointer", "externalTextures" => "Array|Pointer",
    "skinBindings" => "Array|Pointer", "appliedTransform" => "Object|Matrix3", },
    "hkxSkinBinding" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "mesh" => "Pointer", "nodeNames" => "Array|String", "bindPose" =>
    "Array|Object|Matrix4", "initSkinTransform" => "Object|Matrix4", },
    "hkxSparselyAnimatedBool" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "bools" => "Array|Bool", "times" => "Array|F64", },
    "hkxSparselyAnimatedEnum" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "ints" => "Array|I64", "times" => "Array|F64", "enum" =>
    "Pointer", }, "hkxSparselyAnimatedInt" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "ints" => "Array|I64", "times" => "Array|F64", },
    "hkxSparselyAnimatedString" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "strings" => "Array|String", "times" => "Array|F64", },
    "hkxTextureFile" => phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount"
    => "I64", "filename" => "String", "name" => "String", "originalFilename" => "String",
    }, "hkxTextureInplace" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "fileType" => "String", "data" => "Array|U64", "name" =>
    "String", "originalFilename" => "String", }, "hkxTriangleSelectionChannel" =>
    phf_ordered_map! { "memSizeAndFlags" => "U64", "referenceCount" => "I64",
    "selectedTriangles" => "Array|I64", }, "hkxVertexBuffer" => phf_ordered_map! {
    "memSizeAndFlags" => "U64", "referenceCount" => "I64", "data" =>
    "Object|hkxVertexBufferVertexData", "desc" => "Object|hkxVertexDescription", },
    "hkxVertexBufferVertexData" => phf_ordered_map! { "vectorData" =>
    "Array|Object|Vector4", "floatData" => "Array|F64", "uint32Data" => "Array|U64",
    "uint16Data" => "Array|U64", "uint8Data" => "Array|U64", "numVerts" => "U64",
    "vectorStride" => "U64", "floatStride" => "U64", "uint32Stride" => "U64",
    "uint16Stride" => "U64", "uint8Stride" => "U64", }, "hkxVertexDescription" =>
    phf_ordered_map! { "decls" => "Array|Object|hkxVertexDescriptionElementDecl", },
    "hkxVertexDescriptionElementDecl" => phf_ordered_map! { "byteOffset" => "U64", "type"
    => "String", "usage" => "String", "byteStride" => "U64", "numElements" => "U64", },
    "hkxVertexFloatDataChannel" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "perVertexFloats" => "Array|F64", "dimensions" =>
    "String", }, "hkxVertexIntDataChannel" => phf_ordered_map! { "memSizeAndFlags" =>
    "U64", "referenceCount" => "I64", "perVertexInts" => "Array|I64", },
    "hkxVertexSelectionChannel" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "selectedVertices" => "Array|I64", },
    "hkxVertexVectorDataChannel" => phf_ordered_map! { "memSizeAndFlags" => "U64",
    "referenceCount" => "I64", "perVertexVectors" => "Array|Object|Vector4", },
};
/// Find class information by class name.
pub fn find_class_info(class_name: &str) -> Option<&'static FieldInfo> {
    CLASS_TABLE.get(class_name)
}
/// Find a field type from the fields map.
pub fn find_json_parser_by(field_name: &str, fields: &FieldInfo) -> Option<&'static str> {
    fields.get(field_name).map(|v| &**v)
}

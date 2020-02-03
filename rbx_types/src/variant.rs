use crate::{
    Axes, BinaryString, BrickColor, CFrame, Color3, Color3uint8, ColorSequence, EnumValue, Faces,
    NumberRange, NumberSequence, PhysicalProperties, Ray, Rect, Ref, SharedString, UDim, UDim2,
    Vector2, Vector2int16, Vector3, Vector3int16,
};

/// Reduces boilerplate from listing different values of Variant by wrapping
/// them into a macro.
macro_rules! make_variant {
    ( $( $variant_name: ident($inner_type: ty), )* ) => {
        /// Represents any Roblox type. Useful for operating generically on Roblox
        /// instances.
        ///
        /// ## Stability
        ///
        /// New variants may be added to `Variant` in minor releases. As
        /// such, it is marked `#[non_exhaustive]`.
        #[derive(Debug, Clone, PartialEq)]
        #[non_exhaustive]
        #[cfg_attr(
            feature = "serde",
            derive(serde::Serialize, serde::Deserialize),
            serde(tag = "Type", content = "Value")
        )]
        pub enum Variant {
            $(
                $variant_name($inner_type),
            )*
        }

        impl Variant {
            pub fn ty(&self) -> VariantType {
                match self {
                    $(
                        Variant::$variant_name(_) => VariantType::$variant_name,
                    )*
                }
            }
        }

        /// Represents a type that can be held in a `Variant`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        #[cfg_attr(
            feature = "serde",
            derive(serde::Serialize, serde::Deserialize),
        )]
        pub enum VariantType {
            $(
                $variant_name,
            )*
        }

        #[cfg(test)]
        mod generated_test {
            use super::*;

            /// This test makes sure that every type represented in `Variant`
            /// can be converted via `Into` into Variant.
            ///
            /// Because we're just asserting that some code compiles, we don't
            /// need a #[test] annotation, but we do need to indicate that we
            /// don't care that it's unused.
            ///
            /// If we forget to impl From when new types are added to Variant,
            /// this test will start failing.
            #[allow(dead_code)]
            fn conversions_are_exhaustive() {
                /// A dummy trait that lets us lie and say we can construct any
                /// type we want.
                trait FakeConstructable {
                    fn new() -> Self;
                }

                impl<T> FakeConstructable for T {
                    fn new() -> Self {
                        unreachable!()
                    }
                }

                $(
                    let _: Variant = <$inner_type as FakeConstructable>::new().into();
                )*
            }
        }
    };
}

make_variant! {
    Axes(Axes),
    BinaryString(BinaryString),
    BrickColor(BrickColor),
    Bool(bool),
    CFrame(CFrame),
    Color3(Color3),
    Color3uint8(Color3uint8),
    ColorSequence(ColorSequence),
    Content(String),
    EnumValue(EnumValue),
    Faces(Faces),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Int64(i64),
    NumberRange(NumberRange),
    NumberSequence(NumberSequence),
    PhysicalProperties(PhysicalProperties),
    Ray(Ray),
    Rect(Rect),
    Ref(Ref),
    SharedString(SharedString),
    String(String),
    UDim(UDim),
    UDim2(UDim2),
    Vector2(Vector2),
    Vector2int16(Vector2int16),
    Vector3(Vector3),
    Vector3int16(Vector3int16),
}

/// Implement conversions from rbx_types types into the equivalent `Variant`
/// value.
macro_rules! trivial_variant_from {
    ( $( $type: ident, )* ) => {
        $(
            impl From<$type> for Variant {
                fn from(value: $type) -> Self {
                    Self::$type(value)
                }
            }
        )*
    };
}

trivial_variant_from! {
    Axes,
    BinaryString,
    BrickColor,
    CFrame,
    Color3,
    Color3uint8,
    ColorSequence,
    EnumValue,
    Faces,
    NumberRange,
    NumberSequence,
    PhysicalProperties,
    Ray,
    Rect,
    Ref,
    SharedString,
    UDim,
    UDim2,
    Vector2,
    Vector2int16,
    Vector3,
    Vector3int16,
}

/// Implement conversions from common Rust types into their equivalent `Variant`
/// value.
macro_rules! primitive_variant_from {
    ( $( $prim_type: ty => $rbx_type: ident, )* ) => {
        $(
            impl From<$prim_type> for Variant {
                fn from(value: $prim_type) -> Self {
                    Self::$rbx_type(value)
                }
            }
        )*
    };
}

primitive_variant_from! {
    bool => Bool,
    f32 => Float32,
    f64 => Float64,
    i32 => Int32,
    i64 => Int64,
    String => String,
}

#[cfg(all(test, feature = "serde"))]
mod serde_test {
    use super::*;

    #[test]
    fn human() {
        let vec2 = Variant::Vector2(Vector2::new(5.0, 7.0));

        let ser = serde_json::to_string(&vec2).unwrap();
        assert_eq!(ser, r#"{"Type":"Vector2","Value":[5.0,7.0]}"#);

        let de: Variant = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, vec2);
    }

    #[test]
    #[ignore]
    fn non_human() {
        let vec2 = Variant::Vector2(Vector2::new(5.0, 7.0));

        let ser = bincode::serialize(&vec2).unwrap();

        // FIXME: This call currently fails because bincode does not support
        // Deserializer::deserialize_identifier.

        let de: Variant = bincode::deserialize(&ser).unwrap();
        assert_eq!(de, vec2);
    }
}

//! Some image related functions to handle cover image

use crate::errors::{MetadataError, MetadataImageError};
use kmeans_colors::{get_kmeans_hamerly, Kmeans, Sort};
use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::map_res,
    sequence::tuple,
    IResult,
};
use palette::{
    cast::ComponentsAs, rgb::channels::Rgba, white_point::D65, IntoColor, Lab, Srgb, Srgba,
};

/// Gets the dominant colors of an image given as a byte array.
///
/// Returns a vector of up to 5 colors in the format of u32, which is a 32-bit maximum
/// unsigned integer representing an RGB color in the format of 0xRRGGBB.
///
/// The dominant colors are determined by the k-means clustering algorithm.
/// The k value is set to 5, which means that the algorithm will find the 5
/// most prominent colors in the image. The algorithm also sorts the colors
/// by their percentage of the image, so the first color in the vector is
/// the most prominent color in the image.
///
/// If the image is not a valid image, or if the image is not an RGB or RGBA
/// image, the function will return an error.
///
/// If the image is a valid image, but the k-means clustering algorithm fails
/// to find any dominant colors, the function will also return a [`MetadataImageError::NoDominantColor`].
pub fn get_dominant_colors(image_bytes: &[u8]) -> Result<Vec<u32>, MetadataError> {
    let mut lab_cache = fxhash::FxHashMap::default();
    let mut lab_pixels: Vec<Lab<D65, f32>> = Vec::new();

    let img = image::load_from_memory(image_bytes)
        .map_err(|e| MetadataImageError::LoadError(e))?
        .into_rgba8();
    let img_vec: &[Srgba<u8>] = img.as_raw().components_as();

    cached_srgba_to_lab(img_vec.iter(), &mut lab_cache, &mut lab_pixels);

    let mut result = Kmeans::new();
    for i in 0..2 {
        let run_result = get_kmeans_hamerly(5, 20, 5.0, false, &lab_pixels, i);
        if run_result.score < result.score {
            result = run_result;
        }
    }

    let mut colors = Lab::<D65, f32>::sort_indexed_colors(&result.centroids, &result.indices);
    colors.sort_unstable_by(|a, b| (b.percentage).total_cmp(&a.percentage));

    let mut hex_colors: Vec<u32> = vec![];
    if let Some((last, elements)) = colors.split_last() {
        let int_colors: Vec<u32> = elements
            .iter()
            .map(|elem| {
                let col: Srgb = elem.centroid.into_color();
                let fmt = col.into_format::<u8>();
                fmt.into_u32::<Rgba>()
            })
            .collect::<Vec<u32>>();
        // Push all int_colors into hex_colors
        hex_colors.extend(int_colors);
        // Push last color into hex_colors
        let col: Srgb = last.centroid.into_color();
        let fmt = col.into_format::<u8>();
        hex_colors.push(fmt.into_u32::<Rgba>());
    }

    if hex_colors.is_empty() {
        Err(MetadataImageError::NoDominantColor.into())
    } else {
        Ok(hex_colors)
    }
}

/// Optimized conversion of colors from Srgb to Lab using a hashmap for caching
/// of expensive color conversions.
///
/// Additionally, converting from Srgb to Linear Srgb is special-cased in
/// `palette` to use a lookup table which is faster than the regular conversion
/// using `color.into_format().into_color()`.
fn cached_srgba_to_lab<'a>(
    rgb: impl Iterator<Item = &'a Srgba<u8>>,
    map: &mut fxhash::FxHashMap<[u8; 3], Lab<D65, f32>>,
    lab_pixels: &mut Vec<Lab<D65, f32>>,
) {
    lab_pixels.extend(rgb.map(|color| {
        *map.entry([color.red, color.green, color.blue])
            .or_insert_with(|| color.into_linear::<_, f32>().into_color())
    }))
}

fn from_hex(input: &str) -> Result<u8, std::num::ParseIntError> {
    u8::from_str_radix(input, 16)
}

fn is_hex_digit(c: char) -> bool {
    c.is_digit(16)
}

fn hex_primary(input: &str) -> IResult<&str, u8> {
    map_res(take_while_m_n(2, 2, is_hex_digit), from_hex)(input)
}

/// Parse a hex color string into a [`u32`].
///
/// Format should be: `#RRGGBB`.
pub fn hex_to_u32(hex: &str) -> Result<u32, MetadataError> {
    let (input, _) = tag("#")(hex).map_err(|_: nom::Err<nom::error::Error<&str>>| {
        MetadataImageError::InvalidHexColor(hex.to_string())
    })?;
    let (_, (red, green, blue)) = tuple((hex_primary, hex_primary, hex_primary))(input)
        .map_err(|_| MetadataImageError::InvalidHexColor(hex.to_string()))?;

    Ok((red as u32) << 16 | (green as u32) << 8 | blue as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_u32() {
        assert_eq!(hex_to_u32("#E6E6E6").unwrap(), 15132390);
        assert_eq!(hex_to_u32("#E2B7D1").unwrap(), 14858193);
        assert_eq!(hex_to_u32("#485591").unwrap(), 4740497);
    }
}

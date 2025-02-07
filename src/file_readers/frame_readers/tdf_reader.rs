use {
    crate::{
        converters::{
            ConvertableIndex, Frame2RtConverter, Scan2ImConverter,
            Tof2MzConverter,
        },
        file_readers::{
            common::{
                ms_data_blobs::{BinFileReader, ReadableFromBinFile},
                sql_reader::{FrameTable, ReadableFromSql, SqlReader},
            },
            ReadableFrames,
        },
        Frame, FrameType,
    },
    rayon::prelude::*,
    std::path::Path,
};

#[derive(Debug)]
pub struct TDFReader {
    pub path: String,
    pub tdf_sql_reader: SqlReader,
    tdf_bin_reader: BinFileReader,
    pub rt_converter: Frame2RtConverter,
    pub im_converter: Scan2ImConverter,
    pub mz_converter: Tof2MzConverter,
    pub frame_table: FrameTable,
}

impl TDFReader {
    pub fn new(path: &String) -> Self {
        let tdf_sql_reader: SqlReader = SqlReader {
            path: String::from(path),
        };
        let frame_table: FrameTable = FrameTable::from_sql(&tdf_sql_reader);
        let file_name: String = Path::new(&path)
            .join("analysis.tdf_bin")
            .to_string_lossy()
            .to_string();
        let tdf_bin_reader: BinFileReader = BinFileReader::new(
            String::from(&file_name),
            frame_table.offsets.clone(),
        );
        Self {
            path: path.to_string(),
            tdf_bin_reader: tdf_bin_reader,
            rt_converter: Self::get_rt_converter(&frame_table),
            im_converter: Scan2ImConverter::from_sql(&tdf_sql_reader),
            mz_converter: Tof2MzConverter::from_sql(&tdf_sql_reader),
            frame_table: frame_table,
            tdf_sql_reader: tdf_sql_reader,
        }
    }

    fn get_rt_converter(frame_table: &FrameTable) -> Frame2RtConverter {
        let retention_times: Vec<f64> = frame_table.rt.clone();
        Frame2RtConverter::new(retention_times)
    }
}

impl ReadableFrames for TDFReader {
    fn read_single_frame(&self, index: usize) -> Frame {
        let mut frame: Frame =
            Frame::read_from_file(&self.tdf_bin_reader, index);
        frame.rt = self.rt_converter.convert(index as u32);
        frame.index = self.frame_table.id[index];
        let msms_type = self.frame_table.msms_type[index];
        frame.frame_type = match msms_type {
            0 => FrameType::MS1,
            8 => FrameType::MS2DDA,
            9 => FrameType::MS2DIA,
            _ => FrameType::Unknown,
        };
        frame
    }

    fn read_all_frames(&self) -> Vec<Frame> {
        (0..self.tdf_bin_reader.size())
            .into_par_iter()
            .map(|index| self.read_single_frame(index))
            .collect()
    }
}

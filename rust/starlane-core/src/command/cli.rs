use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;
use mesh_portal_serde::version::latest::entity::request::create::{AddressSegmentTemplate, KindTemplate, Template};
use mesh_portal_serde::version::latest::frame::PrimitiveFrame;
use mesh_portal_serde::version::latest::id::Address;
use mesh_portal_serde::version::latest::messaging::Message;
use mesh_portal_serde::version::latest::resource::ResourceStub;
use mesh_portal_tcp_common::{PrimitiveFrameReader, PrimitiveFrameWriter};
use mesh_portal_versions::version::v0_0_1::entity::request::create::AddressTemplate;
use mesh_portal_versions::version::v0_0_1::id::RouteSegment;
use mesh_portal_versions::version::v0_0_1::parse::Res;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use crate::command::cli::outlet::Frame;
use crate::command::execute::CommandExecutor;
use crate::command::parse::command_line;
use crate::error::Error;
use crate::star::shell::sys::SysResource;
use crate::star::StarSkel;
use crate::starlane::api::StarlaneApi;
use crate::starlane::ServiceSelection;


pub mod inlet {
    use std::convert::TryFrom;
    use mesh_portal_serde::version::latest::frame::PrimitiveFrame;
    use serde::{Serialize, Deserialize};
    use crate::error::Error;

    #[derive(Debug,Clone,Serialize,Deserialize)]
    pub enum Frame {
        CommandLine(String)
    }

    impl TryFrom<PrimitiveFrame> for Frame {
        type Error = Error;

        fn try_from(value: PrimitiveFrame) -> Result<Self, Self::Error> {
            Ok(bincode::deserialize(value.data.as_slice() )?)
        }
    }
}

pub mod outlet{
    use std::convert::TryFrom;
    use mesh_portal_serde::version::latest::frame::PrimitiveFrame;
    use serde::{Serialize, Deserialize};
    use crate::error::Error;

    #[derive(Debug,Clone,Serialize,Deserialize)]
    pub enum Frame {
        StdOut(String),
        StdErr(String),
        EndOfCommand(i32)
    }

    impl TryFrom<PrimitiveFrame> for Frame {
        type Error = Error;

        fn try_from(value: PrimitiveFrame) -> Result<Self, Self::Error> {
            Ok(bincode::deserialize(value.data.as_slice() )?)
        }
    }
}

pub struct CliServer {

}


impl CliServer {
    pub async fn new( api: StarlaneApi, mut stream: TcpStream ) -> Result<(),Error> {
        let template = Template {
            address: AddressTemplate {
                parent: Address::root(),
                child_segment_template: AddressSegmentTemplate::Pattern("control-%".to_string())
            },
            kind: KindTemplate {
                resource_type: "Control".to_string(),
                kind: None,
                specific: None
            }
        };

        let (messenger_tx, mut messenger_rx) = mpsc::channel(1024);

        tokio::spawn(async move {
            while let Some(_) = messenger_rx.recv().await {
                // ignore messages for now
            }
        });


        let stub = api.create_sys_resource(template,messenger_tx).await?;

        let (reader,writer) = stream.into_split();

        let mut reader :FrameReader<inlet::Frame> = FrameReader::new( PrimitiveFrameReader::new( reader ));
        let mut writer = FrameWriter::new( PrimitiveFrameWriter::new( writer ));
        let (output_tx,mut output_rx) = mpsc::channel(1024);

        {
            let stub = stub.clone();
            tokio::task::spawn_blocking(move || {
                tokio::spawn(async move {
                    while let Ok(frame) = reader.read().await {
                        match frame {
                            inlet::Frame::CommandLine(line) => {
                                CommandExecutor::execute(line, output_tx.clone(), stub.clone(), api.clone() ).await;
                            }
                        }
                    }
                })
            });
        }

        {
            tokio::task::spawn_blocking(move || {
                tokio::spawn(async move {
                    while let Some(frame) = output_rx.recv().await {
                        writer.write(frame).await;
                    }
                })
            });
        }

        Ok(())
    }
}

pub struct CliClient {
    reader: FrameReader<outlet::Frame>,
    writer: FrameWriter<inlet::Frame>
}

impl CliClient {

    pub fn new( host: String ) -> Result<Self,Error> {
        let mut stream = TcpStream::connect(host.clone()).await?;

        // first select service
        let service = ServiceSelection::Cli.to_string();
        stream.write_u32(service.len() as u32 )?;
        stream.write_all( service.as_bytes() )?;

        let (reader,writer) = stream.into_split();
        let mut reader : FrameReader<outlet::Frame> = FrameReader::new( PrimitiveFrameReader::new( reader ));
        let mut writer : FrameWriter<inlet::Frame>  = FrameWriter::new( PrimitiveFrameWriter::new( writer ));

        Ok(Self {
            reader,
            writer
        })
    }

    pub async fn send( mut self, command_line: String ) -> Result<CommandExchange,Error> {
        let writer = &mut self.writer

        let result = tokio::task::spawn_blocking( move || {
            tokio::spawn(async move {
                writer.write( inlet::Frame::CommandLine(command_line)).await
            } )
        }).await?.await?;

        Ok(self.into())
    }
}

impl Into<CommandExchange> for CliClient {
    fn into(self) -> CommandExchange{
        CommandExchange {
            reader: self.reader,
            writer: self.writer,
            complete: false
        }
    }
}

impl Into<CliClient> for CommandExchange{
    fn into(self) -> CliClient{
        CliClient{
            reader: self.reader,
            writer: self.writer
        }
    }
}


pub struct CommandExchange {
    reader: FrameReader<outlet::Frame>,
    writer: FrameWriter<inlet::Frame>,
    complete: bool
}

impl CommandExchange {
    pub async fn read( &mut self ) -> Option<Result<outlet::Frame,Error>> {
        if self.complete {
            return Option::None;
        }

        let reader = &mut self.reader;
        let frame = tokio::task::spawn_blocking( move || {
           tokio::spawn(async move {
               reader.read().await
           } )
        }).await?.await??;

        if let outlet::Frame::EndOfCommand(code) = frame {
            self.complete = true;
        }

        Option::Some(Ok(frame))
    }
}

pub enum Output {
    StdOut(String),
    StdErr(String),
    End(i32)
}




pub struct FrameWriter<FRAME> where FRAME: TryInto<PrimitiveFrame> {
    stream: PrimitiveFrameWriter,
    phantom: PhantomData<FRAME>
}

impl <FRAME> FrameWriter<FRAME> where FRAME: TryInto<PrimitiveFrame>  {
    pub fn new(stream: PrimitiveFrameWriter) -> Self {
        Self {
            stream,
            phantom: PhantomData
        }
    }
}

impl FrameWriter<outlet::Frame>  {

    pub async fn write( &mut self, frame: outlet::Frame ) -> Result<(),Error> {
        let frame = frame.try_into()?;
        Ok(self.stream.write(frame).await?)
    }

}

impl FrameWriter<inlet::Frame> {

    pub async fn write( &mut self, frame: inlet::Frame ) -> Result<(),Error> {
        let frame = frame.try_into()?;
        Ok(self.stream.write(frame).await?)
    }
}


pub struct FrameReader<FRAME> {
    stream: PrimitiveFrameReader,
    phantom: PhantomData<FRAME>
}

impl <FRAME> FrameReader<FRAME>  {
    pub fn new(stream: PrimitiveFrameReader) -> Self {
        Self {
            stream,
            phantom: PhantomData
        }
    }
}

impl FrameReader<outlet::Frame> {
    pub async fn read( &mut self ) -> Result<outlet::Frame,Error> {
        let frame = self.stream.read().await?;
        Ok(outlet::Frame::try_from(frame)?)
    }
}

impl FrameReader<inlet::Frame> {
    pub async fn read( &mut self ) -> Result<inlet::Frame,Error> {
        let frame = self.stream.read().await?;
        Ok(inlet::Frame::try_from(frame)?)
    }
}
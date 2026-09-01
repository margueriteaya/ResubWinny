<script lang="ts">
  import { onMount } from "svelte";
  let canvas: HTMLCanvasElement;

  const vertexSource = `#version 300 es
    in vec2 aPosition; out vec2 vUv;
    void main(){vUv=vec2(aPosition.x*.5+.5,1.-(aPosition.y*.5+.5));gl_Position=vec4(aPosition,0.,1.);}`;

  const fragmentSource = `#version 300 es
    precision highp float;
    in vec2 vUv; out vec4 outColor; uniform float uDark; uniform float uPageExtent;

    float hash(vec2 p){p=fract(p*vec2(123.34,456.21));p+=dot(p,p+45.32);return fract(p.x*p.y);}
    float noise2d(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x),f.y);}
    float fbm(vec2 p){float v=0.,w=.56;for(int i=0;i<4;i++){v+=noise2d(p)*w;p=p*2.03+vec2(17.1,9.2);w*=.48;}return v;}

    float doubleExp(float t){
      t=clamp(t,0.,1.);const float k=.5;float d=exp(k)-1.;
      if(t<.5)return .5*(exp(k*2.*t)-1.)/d;
      return 1.-.5*(exp(k*2.*(1.-t))-1.)/d;
    }

    float catenaryX(float y,float yEnd,float xStart,float xEnd){
      const float q=1.72;float v=clamp(y/yEnd,0.,1.);
      float t=acosh(1.+v*(cosh(q)-1.))/q;
      return mix(xStart,xEnd,t);
    }

    void main(){
      float pageY=vUv.y*uPageExtent;
      float l1=catenaryX(pageY,.30,1./3.,2./3.);
      float l2=catenaryX(pageY,1./6.,.5,1.);
      float p=clamp((vUv.x-l1)/max(l2-l1,.035),0.,1.);
      float n=fbm(vec2(vUv.x*9.2,pageY*24.));
      float envelope=4.*p*(1.-p);
      p=clamp(p+(n-.5)*.145*envelope,0.,1.);

      vec3 cold=mix(vec3(222.,235.,237.),vec3(18.,31.,33.),uDark)/255.;
      vec3 warm=mix(vec3(228.,225.,234.),vec3(24.,21.,30.),uDark)/255.;
      vec3 color=mix(cold,warm,doubleExp(p));
      float vertical=doubleExp((pageY-.25)/((4./9.)-.25));
      color=mix(color,vec3(1.-uDark),vertical);
      color+=(n-.5)*.009*envelope*(1.-vertical);
      outColor=vec4(clamp(color,0.,1.),1.);
    }`;

  onMount(() => {
    const gl=canvas.getContext("webgl2",{antialias:false,alpha:false,powerPreference:"low-power"});
    if(!gl)return;
    const compile=(type:number,source:string)=>{const shader=gl.createShader(type);if(!shader)return null;gl.shaderSource(shader,source);gl.compileShader(shader);if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS)){console.warn("Onboarding colour shader unavailable",gl.getShaderInfoLog(shader));gl.deleteShader(shader);return null;}return shader;};
    const vertex=compile(gl.VERTEX_SHADER,vertexSource),fragment=compile(gl.FRAGMENT_SHADER,fragmentSource);if(!vertex||!fragment)return;
    const program=gl.createProgram();if(!program)return;gl.attachShader(program,vertex);gl.attachShader(program,fragment);gl.linkProgram(program);gl.deleteShader(vertex);gl.deleteShader(fragment);if(!gl.getProgramParameter(program,gl.LINK_STATUS))return;
    const buffer=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,1,-1,-1,1,1,1]),gl.STATIC_DRAW);
    const position=gl.getAttribLocation(program,"aPosition");gl.enableVertexAttribArray(position);gl.vertexAttribPointer(position,2,gl.FLOAT,false,0,0);const dark=gl.getUniformLocation(program,"uDark"),pageExtent=gl.getUniformLocation(program,"uPageExtent");
    const draw=()=>{const rect=canvas.getBoundingClientRect(),page=canvas.closest<HTMLElement>(".onboarding-page"),pageHeight=Math.max(1,page?.getBoundingClientRect().height??rect.height),dpr=Math.min(devicePixelRatio||1,1.5);canvas.width=Math.max(1,Math.floor(rect.width*dpr));canvas.height=Math.max(1,Math.floor(rect.height*dpr));gl.viewport(0,0,canvas.width,canvas.height);gl.useProgram(program);gl.uniform1f(dark,document.documentElement.dataset.theme==="dark"?1:0);gl.uniform1f(pageExtent,rect.height/pageHeight);gl.drawArrays(gl.TRIANGLE_STRIP,0,4);};
    const observer=new ResizeObserver(draw);observer.observe(canvas);draw();
    const themeObserver=new MutationObserver(draw);themeObserver.observe(document.documentElement,{attributes:true,attributeFilter:["data-theme"]});
    return()=>{observer.disconnect();themeObserver.disconnect();gl.deleteBuffer(buffer);gl.deleteProgram(program);};
  });
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
  canvas{position:absolute;z-index:0;inset:0;display:block;width:100%;height:100%;pointer-events:none}
  @media(forced-colors:active){canvas{display:none}}
</style>

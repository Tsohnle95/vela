export default {
 name:'Soft aperture',family:'Geometry',description:'A delicate opening appears within each cube as the pearl comes near.',rule:'Four small curved brackets open around the projected pearl, controlled by proximity.',
 setup({add,path,faceEffects,set}){const nodes=faceEffects.map(g=>{const n=add(g,'g');for(let j=0;j<4;j++)path(n,'M-27 -60Q-60 -60 -60 -27','#e9dbff',3,{transform:`rotate(${j*90})`});return n;});return ({i,x,y,dx,dy,strength})=>set(nodes[i],{transform:`translate(${x+dx*.23} ${y+dy*.23}) scale(${.55+strength*.45})`,opacity:strength*.3});}
};

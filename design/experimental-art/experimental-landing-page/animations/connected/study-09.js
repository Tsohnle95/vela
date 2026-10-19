export default {
 name:'Prismatic glint',family:'Light',description:'A translucent triangular glint turns inside the cube nearest the pearl.',rule:'The triangle points toward the pearl; its visibility follows proximity.',
 setup({path,faceEffects,set}){const nodes=faceEffects.map(g=>path(g,'M-70 35L0 -70L70 35Z','#f3dcff',2,{fill:'#e1c9ff'}));return ({i,x,y,angle,strength})=>set(nodes[i],{transform:`translate(${x} ${y}) rotate(${angle*180/Math.PI+90})`,opacity:strength*.16});}
};

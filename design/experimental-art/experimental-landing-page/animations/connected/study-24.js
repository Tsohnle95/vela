export default {
 name:'Inner parallax',family:'Geometry',description:'A faint inner facet shifts beneath the surface as the pearl moves around the cube.',rule:'The inset geometry moves opposite the pearl, creating a restrained sense of depth.',
 setup({path,faceEffects,set}){const nodes=faceEffects.map(g=>path(g,'M0 -75L65 -38V38L0 75L-65 38V-38ZM-65 -38L0 0L65 -38M0 0V75','#e4d4ff',3,{fill:'#d0b8ff','fill-opacity':.08}));return ({i,x,y,dx,dy,strength})=>set(nodes[i],{transform:`translate(${x-dx*.1} ${y-dy*.09})`,opacity:strength*.28});}
};
